use anyhow::Result;
use redis::streams::{StreamReadOptions, StreamReadReply};
use redis::{Client, Commands};

const STREAM: &str = "nautilus-stream";
const GROUP: &str = "sonar-group";
const CONSUMER: &str = "consumer-1";

fn main() -> Result<()> {
    println!("🚀 Nautilus consumer avviato");

    let client = Client::open("redis://valkey:6379")?;
    let mut con = client.get_connection()?;

    ensure_group(&mut con)?;

    loop {
        let opts = StreamReadOptions::default()
            .group(GROUP, CONSUMER)
            .count(10)
            .block(0);

        let reply: StreamReadReply =
            con.xread_options(&[STREAM], &[">"], &opts)?;

        for stream_key in reply.keys {
            for stream_id in stream_key.ids {
                let id = stream_id.id;

                if let Some(value) = stream_id.map.get("payload") {
                    let payload: String = redis::from_redis_value(value)?;

                    println!("📩 Evento ricevuto [{}]:", id);
                    println!("{}", payload);

                    let _: i32 = con.xack(STREAM, GROUP, &[id.as_str()])?;

                    println!("✅ ACK inviato [{}]", id);
                }
            }
        }
    }
}

fn ensure_group(con: &mut redis::Connection) -> Result<()> {
    let result: redis::RedisResult<String> = redis::cmd("XGROUP")
        .arg("CREATE")
        .arg(STREAM)
        .arg(GROUP)
        .arg("0")
        .arg("MKSTREAM")
        .query(con);

    match result {
        Ok(_) => {
            println!("✅ Consumer group creato: {}", GROUP);
        }
        Err(err) => {
            if err.to_string().contains("BUSYGROUP") {
                println!("ℹ️ Consumer group già esistente: {}", GROUP);
            } else {
                return Err(err.into());
            }
        }
    }

    Ok(())
}