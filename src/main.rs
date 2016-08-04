extern crate mio;

use mio::*;
use mio::tcp::*;
use std::net::SocketAddr;
use std::collections::HashMap;

// Create a constant
const SERVER_TOKEN: Token = Token(0);

// We need PartialEq for comparing
#[derive(PartialEq)]
enum ConnectionState {
    WaitForMessages,
    ReadyToSend,
}

struct Connection {
    socket: TcpStream,
    state: ConnectionState,
    str_buffer: String,
}

impl Connection {
    fn event_handler(&mut self) {
        match self.state {
            ConnectionState::WaitForMessages => {
                // Read data into buffer array
                let mut buffer = vec![];
                match self.socket.try_read_buf(&mut buffer) {
                    Err(e) => {
                        println!("Error try reading: {}", e);
                    },
                    Ok(None) => {
                        println!("Got nothing to read!");
                    },
                    Ok(Some(len)) => {
                        println!("Read {} bytes.",len);
                        self.str_buffer.push_str(
                            &(String::from_utf8(buffer).unwrap()));
                    }
                }
                // Let's see if string buffer contains newline
                println!("The whole string: {:?}", self.str_buffer);
                match self.str_buffer.find('\n') {
                    Some(idx) => {
                        println!("Found new line: {}", idx);
                        self.state = ConnectionState::ReadyToSend;
                    } 
                    None => {
                        println!("No newline exist.");
                        self.state = ConnectionState::WaitForMessages;
                    }
                }
            }

            ConnectionState::ReadyToSend => {
                println!("It's writable!");
                match self.socket.try_write(&self.str_buffer.as_bytes()) {
                    Err(e) => {
                        println!("Error try reading: {}", e);
                    },

                    Ok(None) => {
                        println!("Write None");
                    },

                    Ok(Some(len)) => {
                        println!("Write {} bytes", len);
                    }
                }
                self.str_buffer.clear();
                self.state = ConnectionState::WaitForMessages;
            }
        }
    }

    fn new(socket: TcpStream) -> Connection{
        Connection {
            socket: socket,
            state: ConnectionState::WaitForMessages,
            str_buffer: String::new()
        }
    }
}

struct EventLoopHandler {
    socket: TcpListener,
    clients: HashMap<Token, Connection>,
    token_counter: usize
}

impl Handler for EventLoopHandler {
    // Special traits
    type Timeout = usize;
    type Message = ();
    // Handler function
    fn ready(&mut self, event_loop: &mut EventLoop<EventLoopHandler>,
             token: Token, events: EventSet)
    {
        // Readable or Writable events
        if events.is_readable() || events.is_writable() {
            println!("EventLoop: Read/Write event caught");
            match token {
                SERVER_TOKEN => {
                    println!("EventLoop: Server socket event.");
                    let client_socket = match self.socket.accept() {
                        Err(e) => {
                            println!("Accept error: {}", e);
                            return;
                        },
                        Ok(None) => unreachable!("Accept has returned 'None'"),
                        Ok(Some((sock, addr))) => {
                            println!("EventLoop: Connection from {} accepted.", addr);
                            sock
                        }
                    };

                    self.token_counter += 1;
                    let new_token = Token(self.token_counter);

                    self.clients.insert(new_token, Connection::new(client_socket));
                    println!("EventLoop: Registering new socket.");
                    event_loop.register(&self.clients[&new_token].socket,
                                        new_token,
                                        EventSet::readable() | EventSet::hup(),
                                        PollOpt::edge() | PollOpt::oneshot())
                                        .unwrap();
                }

                token => {
                    println!("EventLoop: Client socket event.");
                    let mut client = self.clients.get_mut(&token).unwrap();
                    client.event_handler();
                    match client.state {
                        ConnectionState::ReadyToSend => {
                            event_loop.reregister(&client.socket,
                                                token,
                                                EventSet::writable() | EventSet::hup(),
                                                PollOpt::edge() | PollOpt::oneshot())
                                                .unwrap();
                        }, 
                        ConnectionState::WaitForMessages => {
                            event_loop.reregister(&client.socket,
                                                token,
                                                EventSet::readable() | EventSet::hup(),
                                                PollOpt::edge() | PollOpt::oneshot())
                                                .unwrap();
                        }
                    }
                }
            }
        }
        // Disconnected
        if events.is_hup() {
            println!("EventLoop: Hup event caught");
            let client = self.clients.remove(&token).unwrap();
            event_loop.deregister(&client.socket).unwrap();
            println!("EventLoop: {} disconnected",
                     client.socket.peer_addr().unwrap());
        }
    }
}

fn main() {
    let mut event_loop = EventLoop::new().unwrap();

    let address = "0.0.0.0:10000".parse::<SocketAddr>().unwrap();
    let server_socket = TcpListener::bind(&address).unwrap();

    let mut server = EventLoopHandler {
        token_counter: 1,        // Starting the token counter from 1
        clients: HashMap::new(), // Creating an empty HashMap
        socket: server_socket    // Handling the ownership of the socket to the struct
    };

    event_loop.register(&server.socket,
                        SERVER_TOKEN,
                        EventSet::readable(),
                        PollOpt::edge()).unwrap();

    println!("Start the event loop.");
    event_loop.run(&mut server).unwrap();
}
