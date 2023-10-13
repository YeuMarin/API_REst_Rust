use postgres::{ Client, NoTls };
use postgres::Error as PostgresError;
use std::net::{ TcpListener, TcpStream };
use std::io::{ Read, Write };
use std::env;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize, Deserialize)]
struct Helado {
    id: Option<i32>,
    sabor: String,
    precio: String,
}

//DATABASE_URL
const DB_URL: &str = env!("DATABASE_URL");

//constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

fn main(){
    //Set database
    if let Err(e) = set_database() {
        println!("Error: {}", e);
        return;
    }

    
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");

    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}

//handle_client function
fn handle_client(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let mut request = String::new();

    match stream.read(&mut buffer) {
        Ok(size) => {
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line, content) = match &*request {
                r if r.starts_with("POST /helados") => handle_post_request(r),
                r if r.starts_with("GET /helados/") => handle_get_request(r),
                r if r.starts_with("GET /helados") => handle_get_all_request(r),
                r if r.starts_with("PUT /helados/") => handle_put_request(r),
                r if r.starts_with("DELETE /helados/") => handle_delete_request(r),
                _ => (NOT_FOUND.to_string(), "404 Not Found".to_string()),
            };

            stream.write_all(format!("{}{}", status_line, content).as_bytes()).unwrap();
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

//POST
fn handle_post_request(request: &str) -> (String, String) {
    match (get_helado_request_body(&request), Client::connect(DB_URL, NoTls)) {
        (Ok(Helado), Ok(mut client)) => {
            client
                .execute(
                    "INSERT INTO helados (sabor, precio) VALUES ($1, $2)",
                    &[&Helado.sabor, &Helado.precio]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "Helado created".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//GET
fn handle_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) =>
            match client.query_one("SELECT * FROM helados WHERE id = $1", &[&id]) {
                Ok(row) => {
                    let helado = Helado {
                        id: row.get(0),
                        sabor: row.get(1),
                        precio: row.get(2),
                    };

                    (OK_RESPONSE.to_string(), serde_json::to_string(&helado).unwrap())
                }
                _ => (NOT_FOUND.to_string(), "helado not found".to_string()),
            }

        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//GET ALL
fn handle_get_all_request(request: &str) -> (String, String) {
    match Client::connect(DB_URL, NoTls) {
        Ok(mut client) => {
            let mut helados = Vec::new();

            for row in client.query("SELECT * FROM helados", &[]).unwrap() {
                helados.push(Helado {
                    id: row.get(0),
                    sabor: row.get(1),
                    precio: row.get(2),
                });
            }

            (OK_RESPONSE.to_string(), serde_json::to_string(&helados).unwrap())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//PUT
fn handle_put_request(request: &str) -> (String, String) {
    match
        (
            get_id(&request).parse::<i32>(),
            get_helado_request_body(&request),
            Client::connect(DB_URL, NoTls),
        )
    {
        (Ok(id), Ok(helado), Ok(mut client)) => {
            client
                .execute(
                    "UPDATE helados SET sabor = $1, precio = $2 WHERE id = $3",
                    &[&helado.sabor, &helado.precio, &id]
                )
                .unwrap();

            (OK_RESPONSE.to_string(), "Helado updated".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//DELETE
fn handle_delete_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(id), Ok(mut client)) => {
            let rows_affected = client.execute("DELETE FROM helados WHERE id = $1", &[&id]).unwrap();

            if rows_affected == 0 {
                return (NOT_FOUND.to_string(), "Helado not found".to_string());
            }

            (OK_RESPONSE.to_string(), "Helado deleted".to_string())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

//set_database 
fn set_database() -> Result<(), PostgresError> {
    //Connect to database
    let mut client = Client::connect(DB_URL, NoTls)?;

    //Create table
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS helados (
            id SERIAL PRIMARY KEY,
            sabor VARCHAR NOT NULL,
            precio VARCHAR NOT NULL
        )"
    )?;
    Ok(())
}

//get_id function
fn get_id(request: &str) -> &str {
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

//deserialize user from request body with the id
fn get_helado_request_body(request: &str) -> Result<Helado, serde_json::Error> {
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}