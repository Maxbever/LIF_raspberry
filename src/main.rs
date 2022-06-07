use std::{thread, time};
use std::sync::{Arc, Mutex};
use local_ip_address::local_ip;
use rustupolis_server::client::Client;
use rustupolis_server::repository::Repository;
use rustupolis_server::server::{Protocol, Server};
use rustupolis_server::server_launcher::ServerLauncher;
use rustupolis_server::{tuple, E};

fn main() {
    // Create server
    let ip_address = local_ip().unwrap().to_string();
    let port_tcp = String::from("9000");
    let port_udp = String::from("9001");

    let repository = Repository::new("admin");

    repository.add_tuple_space(String::from("DATA"), vec![String::from("admin")]);

    let server_tcp = Server::new(Protocol::TCP, &ip_address, &port_tcp, &repository);
    let server_udp = Server::new(Protocol::UDP, &ip_address, &port_udp, &repository);

    let server_launcher = ServerLauncher::new(vec![server_tcp, server_udp]);

    // Get data from mobile
    let clients = vec![("Client 1", "192.168.0.4", 9000)];
    let repo = Arc::new(Mutex::new(&repository));
    dbg!("Start reaching data");

    crossbeam::scope(|scope| {
        scope.spawn(|_| {
            server_launcher.launch_server();
        });

        for mobile in clients {
            let repo2 = repo.clone();
            scope.spawn(move |_| {
                let tuple_space_name = String::from("GPS_DATA");
                let attribute = String::from("admin");
                let mut client = Client::new();
                let (name, ip_adr, port) = mobile;
                client.connect(
                    String::from(ip_adr),
                    port.to_string(),
                    String::from("tcp"),
                    &String::from(name),
                );

                client.attach(
                    &String::from(name),
                    vec![attribute.clone()],
                    &tuple_space_name,
                );
                loop {
                    let mut data = client.in_instr(vec![
                        tuple![E::Any],
                        tuple![E::Any],
                        tuple![E::Any],
                        tuple![E::Any],
                        tuple![E::Any],
                    ]);
                    let mut sum_light = 0.0;
                    let mut nb_tuple = 0;
                    let mut location: (f64, f64) = (50.4, 19.52);

                    while !data.is_empty() {
                        if let E::T(tuple) = data.first() {
                            if let E::D(nbr) = tuple.rest().rest().first() {
                                sum_light += nbr;
                                nb_tuple += 1;
                                if data.rest().is_empty() {
                                    if let E::D(latitude) = tuple.first(){
                                        if let E::D(longitude) = tuple.rest().first() {
                                            location = (*latitude, *longitude)
                                        }
                                    }
                                }
                                data = data.rest();
                            }
                        }else if let E::D(nbr) = data.first() {
                            sum_light += nbr;
                            nb_tuple += 1;
                            if let E::D(latitude) = data.first(){
                                if let E::D(longitude) = data.rest().first() {
                                    location = (*latitude, *longitude)
                                }
                            }
                            break;
                        }
                    }

                    let mean: f64 = (sum_light) as f64 / (nb_tuple) as f64;

                    repo2.lock().unwrap().add_tuple_to_tuple_space(
                        String::from("DATA"),
                        tuple![E::str(name), E::D(location.0), E::D(location.1), E::D(mean)],
                    );

                    thread::sleep(time::Duration::from_secs(10));
                }
            });
        }
    })
    .unwrap();
}
