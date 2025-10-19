use std::io;
use std::time::Duration;

use mlua::{Lua, Result as LuaResult, Table, UserData, UserDataMethods, Value as LuaValue};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

#[derive(Debug)]
struct LuaTcpServer {
    listener: TcpListener,
    accept_timeout: Option<Duration>,
}

#[derive(Debug)]
struct LuaTcpClient {
    stream: TcpStream,
    rw_timeout: Option<Duration>, // both read & write for simplicity
}

/* ------------------------- Server methods ------------------------- */

impl UserData for LuaTcpServer {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // server:settimeout(seconds | nil)
        methods.add_method_mut("settimeout", |_, this, secs: Option<f64>| {
            this.accept_timeout = secs.map(|s| {
                if s <= 0.0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_secs_f64(s)
                }
            });
            Ok(())
        });

        // server:accept() -> client | nil, "timeout"
        methods.add_async_method("accept", |lua, this, ()| async move {
            let fut = this.listener.accept();
            let res = if let Some(t) = this.accept_timeout {
                match timeout(t, fut).await {
                    Ok(r) => r,
                    Err(_) => {
                        return Ok((
                            LuaValue::Nil,
                            LuaValue::String(lua.create_string("timeout")?),
                        ));
                    }
                }
            } else {
                fut.await
            };

            match res {
                Ok((stream, _addr)) => {
                    let client = LuaTcpClient {
                        stream,
                        rw_timeout: None,
                    };
                    // Return userdata directly; no .into()
                    Ok((
                        LuaValue::UserData(lua.create_userdata(client)?),
                        LuaValue::Nil,
                    ))
                }
                Err(e) => Err(mlua::Error::external(e)),
            }
        });

        methods.add_method_mut("close", |_, _this, ()| {
            // drop on GC; nothing to do
            Ok(())
        });
    }
}

/* ------------------------- Client methods ------------------------- */

impl UserData for LuaTcpClient {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // client:settimeout(seconds | nil)
        methods.add_method_mut("settimeout", |_, this, secs: Option<f64>| {
            this.rw_timeout = secs.map(|s| {
                if s <= 0.0 {
                    Duration::from_millis(1)
                } else {
                    Duration::from_secs_f64(s)
                }
            });
            Ok(())
        });

        // client:receive(mode) -> string | nil, "timeout"
        // modes: "*l" = line (no trailing \n), "*a" = all, "<number>" bytes
        methods.add_async_method_mut("receive", |lua, mut this, mode: LuaValue| async move {
            // Parse mode
            enum Mode {
                Line,
                All,
                Bytes(usize),
            }
            let mode = match mode {
                LuaValue::String(s) => {
                    let m = s.to_str()?;
                    if m == "*l" {
                        Mode::Line
                    } else if m == "*a" {
                        Mode::All
                    } else {
                        return Err(mlua::Error::external(
                            "receive: expected \"*l\", \"*a\", or a byte count number",
                        ));
                    }
                }
                LuaValue::Integer(n) if n > 0 => Mode::Bytes(n as usize),
                LuaValue::Number(n) if n.is_sign_positive() && n.fract() == 0.0 && n > 0.0 => {
                    Mode::Bytes(n as usize)
                }
                _ => return Err(mlua::Error::external("receive: bad mode")),
            };

            // Cache timeout BEFORE constructing the future that borrows &mut this
            let timeout_opt = this.rw_timeout;

            // Inner read future (so we can wrap with timeout if needed)
            let read_fut = async {
                match mode {
                    Mode::Line => {
                        // Read until '\n' (CRLF friendly: strip trailing \r\n or \n)
                        let mut buf = Vec::with_capacity(128);
                        let mut byte = [0u8; 1];
                        loop {
                            let n = this.stream.read(&mut byte).await?;
                            if n == 0 {
                                break;
                            } // EOF
                            buf.push(byte[0]);
                            if byte[0] == b'\n' {
                                break;
                            }
                            // Guard against pathological growth
                            if buf.len() > 8 * 1024 * 1024 {
                                return Err(io::Error::other("line too long"));
                            }
                        }
                        if buf.ends_with(b"\r\n") {
                            buf.truncate(buf.len() - 2);
                        } else if buf.last() == Some(&b'\n') {
                            buf.pop();
                        }
                        Ok::<_, io::Error>(buf)
                    }
                    Mode::All => {
                        let mut buf = Vec::new();
                        let mut chunk = [0u8; 4096];
                        loop {
                            let n = this.stream.read(&mut chunk).await?;
                            if n == 0 {
                                break;
                            }
                            buf.extend_from_slice(&chunk[..n]);
                            if buf.len() > 16 * 1024 * 1024 {
                                return Err(io::Error::other("too large"));
                            }
                        }
                        Ok(buf)
                    }
                    Mode::Bytes(nwant) => {
                        let mut buf = vec![0u8; nwant];
                        let mut off = 0;
                        while off < nwant {
                            let n = this.stream.read(&mut buf[off..]).await?;
                            if n == 0 {
                                buf.truncate(off);
                                break;
                            } // EOF
                            off += n;
                        }
                        buf.truncate(off);
                        Ok(buf)
                    }
                }
            };

            let out: Vec<u8> = if let Some(t) = timeout_opt {
                match timeout(t, read_fut).await {
                    Ok(r) => r.map_err(mlua::Error::external)?,
                    Err(_) => {
                        return Ok((
                            LuaValue::Nil,
                            LuaValue::String(lua.create_string("timeout")?),
                        ));
                    }
                }
            } else {
                read_fut.await.map_err(mlua::Error::external)?
            };

            Ok((LuaValue::String(lua.create_string(&out)?), LuaValue::Nil))
        });

        // client:send(data) -> bytes_sent | nil, "timeout"
        methods.add_async_method_mut("send", |lua, mut this, data: LuaValue| async move {
            let bytes: Vec<u8> = match data {
                LuaValue::String(s) => s.as_bytes().to_vec(),
                LuaValue::Table(t) => {
                    // (optional) allow {byte,...}
                    let mut v = Vec::new();
                    for pair in t.sequence_values::<u8>() {
                        v.push(pair?);
                    }
                    v
                }
                _ => {
                    return Err(mlua::Error::external(
                        "send: expected string or byte-array table",
                    ));
                }
            };

            // Cache timeout BEFORE constructing the future that borrows &mut this
            let timeout_opt = this.rw_timeout;

            let write_fut = async {
                this.stream.write_all(&bytes).await?;
                this.stream.flush().await?;
                Ok::<usize, io::Error>(bytes.len())
            };

            let n = if let Some(t) = timeout_opt {
                match timeout(t, write_fut).await {
                    Ok(r) => r.map_err(mlua::Error::external)?,
                    Err(_) => {
                        return Ok((
                            LuaValue::Nil,
                            LuaValue::String(lua.create_string("timeout")?),
                        ));
                    }
                }
            } else {
                write_fut.await.map_err(mlua::Error::external)?
            };

            Ok((LuaValue::Integer(n as i64), LuaValue::Nil))
        });

        // client:close()
        methods.add_async_method_mut("close", |_lua, mut this, ()| async move {
            let _ = this.stream.shutdown().await;
            Ok(())
        });
    }
}

/* ------------------------- Module preload ------------------------- */

pub fn install_socket_preload(lua: &Lua) -> LuaResult<()> {
    let pkg: Table = lua.globals().get("package")?;
    let preload: Table = pkg.get("preload")?;

    let loader = lua.create_function(|lua, ()| {
        let m = lua.create_table()?;

        // socket.bind(ip, port) -> server
        let bind_fn = lua.create_async_function(|lua, (ip, port): (String, u16)| async move {
            let addr = format!("{}:{}", ip, port);
            let listener = TcpListener::bind(&addr)
                .await
                .map_err(mlua::Error::external)?;
            let server = LuaTcpServer {
                listener,
                accept_timeout: None,
            };
            let ud = lua.create_userdata(server)?;
            Ok(ud)
        })?;

        m.set("bind", bind_fn)?;
        Ok(m)
    })?;

    preload.set("yal.socket", loader)?;
    Ok(())
}
