use anyhow::anyhow;
use futures::TryStreamExt;
use mlua::{Error as LuaError, Lua, LuaSerdeExt, Result as LuaResult, Table, Value};
use parking_lot::Mutex;
use reqwest::{Client, Method, StatusCode, redirect::Policy};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{fs::File, io::AsyncWriteExt, sync::Semaphore};

#[derive(Clone)]
pub struct HttpLimits {
    pub max_concurrent: usize,
    pub default_timeout_ms: u64,
    pub default_max_body_bytes: usize,
    pub default_max_redirects: usize,
}

impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            max_concurrent: 16,
            default_timeout_ms: 10_000,
            default_max_body_bytes: 4 * 1024 * 1024, // 4MB
            default_max_redirects: 5,
        }
    }
}

#[derive(Clone)]
pub struct HttpEnv {
    client: Client,
    limits: HttpLimits,
    gate: Arc<Semaphore>,
    default_headers: Arc<Mutex<HashMap<String, String>>>,
}

impl HttpEnv {
    pub fn new(limits: HttpLimits) -> anyhow::Result<Self> {
        let client = Client::builder()
            .redirect(Policy::limited(limits.default_max_redirects))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .pool_idle_timeout(Some(Duration::from_secs(60)))
            .build()?;

        Ok(Self {
            client,
            limits: limits.clone(),
            gate: Arc::new(Semaphore::new(limits.max_concurrent)),
            default_headers: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}

#[derive(Debug, Clone)]
struct RequestOpts {
    method: Method,
    url: String,
    headers: HashMap<String, String>,
    query: HashMap<String, String>,
    timeout_ms: u64,
    max_body_bytes: usize,
    max_redirects: usize,
    body_text: Option<String>,
    body_json: Option<serde_json::Value>,
    body_bytes: Option<Vec<u8>>,
    save_to: Option<String>,
}

fn lua_table_to_map(v: Option<Value>) -> LuaResult<HashMap<String, String>> {
    let mut out = HashMap::new();
    if let Some(Value::Table(t)) = v {
        for pair in t.pairs::<Value, Value>() {
            let (k, v) = pair?;
            let key = match k {
                Value::String(s) => s.to_str()?.to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Number(n) => {
                    if n.fract() == 0.0 {
                        (n as i64).to_string()
                    } else {
                        n.to_string()
                    }
                }
                _ => {
                    return Err(LuaError::external(
                        "header/query keys must be string/number",
                    ));
                }
            };
            let val = match v {
                Value::String(s) => s.to_str()?.to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Boolean(b) => if b { "true" } else { "false" }.to_string(),
                _ => return Err(LuaError::external("header/query values must be scalar")),
            };
            out.insert(key, val);
        }
    }
    Ok(out)
}

fn parse_method(s: &str) -> LuaResult<Method> {
    Method::from_bytes(s.as_bytes()).map_err(LuaError::external)
}

async fn execute_request(
    env: HttpEnv,
    opts: RequestOpts,
) -> anyhow::Result<(StatusCode, HashMap<String, String>, Vec<u8>)> {
    let _permit = env.gate.acquire().await.unwrap();

    // Optionally override redirect policy per-request
    let client = if opts.max_redirects != env.limits.default_max_redirects {
        Client::builder()
            .redirect(Policy::limited(opts.max_redirects))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .pool_idle_timeout(Some(Duration::from_secs(60)))
            .build()?
    } else {
        env.client.clone()
    };

    let mut req = client
        .request(opts.method.clone(), &opts.url)
        .timeout(Duration::from_millis(opts.timeout_ms));

    // default headers first
    {
        let defaults = env.default_headers.lock();
        for (k, v) in defaults.iter() {
            req = req.header(k, v);
        }
    }
    for (k, v) in opts.headers.iter() {
        req = req.header(k, v);
    }

    if !opts.query.is_empty() {
        req = req.query(&opts.query);
    }

    if let Some(j) = opts.body_json {
        req = req.json(&j);
    } else if let Some(t) = opts.body_text {
        req = req.body(t);
    } else if let Some(b) = opts.body_bytes {
        req = req.body(b);
    }

    let resp = req.send().await?;
    let status = resp.status();

    let mut headers_out = HashMap::new();
    for (k, v) in resp.headers().iter() {
        headers_out.insert(
            k.as_str().to_string(),
            v.to_str().unwrap_or_default().to_string(),
        );
    }

    if let Some(path) = opts.save_to {
        let mut file = File::create(&path).await?;
        let mut size: usize = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.try_next().await? {
            size += chunk.len();
            if size > opts.max_body_bytes {
                return Err(anyhow!("body exceeds max_body_bytes"));
            }
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        Ok((status, headers_out, Vec::new()))
    } else {
        let mut body: Vec<u8> = Vec::with_capacity(8192);
        let mut size: usize = 0;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.try_next().await? {
            size += chunk.len();
            if size > opts.max_body_bytes {
                return Err(anyhow!("body exceeds max_body_bytes"));
            }
            body.extend_from_slice(&chunk);
        }
        Ok((status, headers_out, body))
    }
}

pub fn install_http_preload(lua: &Lua, env: HttpEnv) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;
    let env_arc = std::sync::Arc::new(env);

    let loader = {
        let env_arc = env_arc.clone();
        lua.create_function(move |lua, ()| {
            let m = lua.create_table()?;

            // --- request(opts) -------------------------------------------------
            let env_req = env_arc.clone();
            let request_fn: mlua::Function =
                lua.create_async_function(move |lua, opts: Value| {
                    log::info!("http.request called");
                    log::info!(
                        "{}",
                        serde_json::to_string_pretty(&opts).unwrap_or_default(),
                    );
                    let env_req = env_req.clone();
                    async move {
                        // parse opts table
                        let t = match opts {
                            Value::Table(t) => t,
                            _ => {
                                return Err(mlua::Error::external(
                                    "yal.http.request expects a table",
                                ));
                            }
                        };

                        // method/url
                        let method = t
                            .get::<Option<String>>("method")?
                            .unwrap_or_else(|| "GET".to_string());
                        let url: String = t.get("url")?;

                        // headers/query
                        let headers = lua_table_to_map(t.get::<Option<Value>>("headers")?)?;
                        let query = lua_table_to_map(t.get::<Option<Value>>("query")?)?;

                        // limits
                        let timeout_ms = t
                            .get::<Option<u64>>("timeout_ms")?
                            .unwrap_or(env_req.limits.default_timeout_ms);
                        let max_body_bytes = t
                            .get::<Option<usize>>("max_body_bytes")?
                            .unwrap_or(env_req.limits.default_max_body_bytes);
                        let max_redirects = t
                            .get::<Option<usize>>("max_redirects")?
                            .unwrap_or(env_req.limits.default_max_redirects);

                        // bodies
                        let body_text: Option<String> = t.get("body")?;
                        let body_bytes: Option<Vec<u8>> = t.get("body_bytes")?;
                        let body_json_val: Option<Value> = t.get("json")?;
                        let body_json = if let Some(v) = body_json_val {
                            Some(lua.from_value::<serde_json::Value>(v)?)
                        } else {
                            None
                        };
                        let save_to: Option<String> = t.get("save_to")?;

                        let ropts = RequestOpts {
                            method: parse_method(&method)?,
                            url,
                            headers,
                            query,
                            timeout_ms,
                            max_body_bytes,
                            max_redirects,
                            body_text,
                            body_json,
                            body_bytes,
                            save_to,
                        };

                        let (status, headers_out, body) =
                            execute_request((*env_req).clone(), ropts)
                                .await
                                .map_err(mlua::Error::external)?;

                        // build result table
                        let res = lua.create_table()?;
                        res.set("status", status.as_u16())?;

                        let htab = lua.create_table()?;
                        for (k, v) in headers_out {
                            htab.set(k, v)?;
                        }
                        res.set("headers", htab)?;

                        if !body.is_empty() {
                            res.set("body", lua.create_string(&body)?)?;
                        }
                        Ok(res)
                    }
                })?;

            // expose request
            m.set("request", request_fn.clone())?;

            // --- get(url, opts?) -> calls request() ---------------------------
            let request_for_get = request_fn.clone();
            let get_fn =
                lua.create_async_function(move |lua, (url, opts): (String, Option<Table>)| {
                    let request_for_get = request_for_get.clone();
                    async move {
                        let t = lua.create_table()?;
                        t.set("method", "GET")?;
                        t.set("url", url)?;
                        if let Some(o) = opts {
                            for pair in o.pairs::<Value, Value>() {
                                let (k, v) = pair?;
                                t.set(k, v)?;
                            }
                        }
                        request_for_get.call_async::<Table>(t).await
                    }
                })?;
            m.set("get", get_fn)?;

            // --- post_json(url, lua_val, opts?) -> calls request() ------------
            let request_for_post = request_fn.clone();
            let post_json_fn = lua.create_async_function(
                move |lua, (url, body, opts): (String, Value, Option<Table>)| {
                    let request_for_post = request_for_post.clone();
                    async move {
                        let t = lua.create_table()?;
                        t.set("method", "POST")?;
                        t.set("url", url)?;
                        t.set("json", body)?;
                        if let Some(o) = opts {
                            for pair in o.pairs::<Value, Value>() {
                                let (k, v) = pair?;
                                t.set(k, v)?;
                            }
                        }
                        request_for_post.call_async::<Table>(t).await
                    }
                },
            )?;
            m.set("post_json", post_json_fn)?;

            // --- set_default_header(k, v) -------------------------------------
            let env_hdr = env_arc.clone();
            let set_hdr = lua.create_function(move |_, (k, v): (String, String)| {
                env_hdr.default_headers.lock().insert(k, v);
                Ok(())
            })?;
            m.set("set_default_header", set_hdr)?;

            Ok(m)
        })
    }?;

    preload.set("yal.http", loader)?;
    Ok(())
}
