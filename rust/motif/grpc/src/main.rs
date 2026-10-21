// Copyright 2024 Two Six Technologies.

use std::env;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

use ta1_sensor::sensor_provider_server::{SensorProvider, SensorProviderServer};
use ta1_sensor::{
    AlertConfig, MeasurementParams, SensorAsyncCallReturnStatus, SensorAsyncCallReturnStatusCode,
    SensorParams, SensorPlaceholderMsg, SensorProviderC2Received, SensorSendProviderC2Results,
};
use ta1_sensor::sensor_manager_client::SensorManagerClient;
use ta1_sensor::MeasurementProperties;
use ta1_sensor::measurement_properties::MeasurementType1;

use tokio;
use simple_logger::SimpleLogger; // https://docs.rs/simple_logger/latest/simple_logger/
// use client::clientlib;

use log::{
    debug,
    // info,
    warn,
    error};

pub mod ta1_sensor {
    // Need to explicity define package name in proto for this to work.
    tonic::include_proto!("ta1_sensor");
    // NOTE: have to do it this way if there's no package name explicitly specified in the proto.
    // WARN: this isn't working now. The output file gets named `_.rs` if there is no package name
    //       in the proto file.
    // include!(concat!(env!("OUT_DIR"), "/ta1_sensor.rs"));
}

#[derive(Debug)]
pub struct MotifSensorProvider {
    // stub: Arc<Mutex<SensorManagerClient<tonic::transport::Channel>>>
    stub: Arc<tokio::sync::Mutex<SensorManagerClient<tonic::transport::Channel>>>,
    // stub: Arc<SensorManagerClient<tonic::transport::Channel>>
    // /// Boolean flag to signal if a test is already in progress and avoid running multiple tests
    // /// at the same time.
    // test_permits: tokio::sync::Semaphore,
}

impl MotifSensorProvider {
    fn new(stub: Arc<tokio::sync::Mutex<SensorManagerClient<tonic::transport::Channel>>>) -> Self {
        Self {
            stub,
            // test_permits: tokio::sync::Semaphore::new(1),
        }
    }
}

#[tonic::async_trait]
impl SensorProvider for MotifSensorProvider {
    /// Need to implement
    async fn initialize(
        &self,
        request: Request<SensorParams>,
    ) -> Result<Response<SensorAsyncCallReturnStatus>, Status> {
        // Notes from STR: SensorParams is just JSON, and be whatever we want it to be in order to
        // initialize our sensor.
        debug!("initialize: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Success as i32,
        }));
    }
    /// Need to implement
    async fn get_measurement_properties(
        &self,
        request: Request<SensorPlaceholderMsg>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("get_measurement_properties: {:?}", request);

        // let permit_attempt = self.test_permits.try_acquire();
        // match permit_attempt.err() {
        //     Some(_) => {
        //         error!("[ERROR] get_measurement_properties: motif test already in progress.");
        //         return Ok(Response::new(SensorAsyncCallReturnStatus {
        //             status: SensorAsyncCallReturnStatusCode::TypeInternalError as i32,
        //         }));
        //     },
        //     _ => (),
        // }

        // Spawn a thread to do this work so we don't block the calling stub.
        // TODO: really need to audit the async / tokio stuff to make sure I'm not doing something stupid.
        let stub = self.stub.clone();
        tokio::spawn(async move {
            // for i in 1..10000 {
            //     debug!("getting there {}", i);
            //     tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            // }

            // permit_attempt;

            let mut dir = match env::current_exe() {
                Ok(dir) => dir,
                Err(_) => std::path::PathBuf::from("./"),
            };
            dir.pop();
            dir.push("../../client/input/webrtc-5.json");
            // dir.push("../../client/input/webrtc-6.json");
            // dir.push("../../client/input/webrtc-7.json");
            
            let did_succeed = match clientlib::run_input(dir.to_str().unwrap(), false, false) {
                Ok(_) => true,
                Err(_) => false,
            };

            debug!("Did we succeed?! {}", if did_succeed { "Yes!" } else { "No :(" });

            let did_succeed = if did_succeed { 1 } else { -1 };

            let request = tonic::Request::new(MeasurementProperties {
                measurement_id: "motif".into(),
                keyvalue: Some(MeasurementType1 {
                    id: "Motif success boolean".into(),
                    value_1: did_succeed as i64,
                    value_2: did_succeed as i32,
                    value_3: 3789789, //did_succeed as i64,
                    value_4: did_succeed as i32,
                    value_5: did_succeed as i64,
                }),
                measurement_description: "Boolean value indicating if motif succeeded".into(),
                applicable_techniques: vec!["TODO: What do I put here?! Why is it when I set values to zero they don't get sent?".into()],
            });
    
            debug!("calling publish_measurement...");
            
            // TODO: is there a better, more efficient approach as opposed to just locking?
            // https://tokio.rs/tokio/tutorial/shared-state#use-tokios-asynchronous-mutex
            let mut stub = stub.lock().await;
            match stub.publish_measurement(request).await {
                Err(error) => {
                    error!("failed to publish measurement: {}", error);
                }
                _ => (),
            }

            
            debug!("call to publish_measurement complete");
        });


        debug!("get_measurement_properties returned");
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Success as i32,
        }));
    }
    async fn configure_sensor_request(
        &self,
        request: Request<SensorParams>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("configure_sensor_request: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
    /// Need to implement
    async fn configure_measurement_request(
        &self,
        request: Request<MeasurementParams>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("configure_measurement_request: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
    /// Need to implement
    async fn configure_measurement_alerts(
        &self,
        request: Request<AlertConfig>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("configure_measurement_alerts: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
    async fn send_c2_sensor_result(
        &self,
        request: Request<SensorSendProviderC2Results>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("send_c2_sensor_result: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
    async fn on_sensor_c2_received(
        &self,
        request: Request<SensorProviderC2Received>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("on_sensor_c2_received: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
    async fn async_error(
        &self,
        request: Request<SensorPlaceholderMsg>,
    ) -> std::result::Result<tonic::Response<SensorAsyncCallReturnStatus>, tonic::Status> {
        debug!("async_error: {:?}", request);
        return Ok(Response::new(SensorAsyncCallReturnStatus {
            status: SensorAsyncCallReturnStatusCode::Unsupported as i32,
        }));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().env().init().unwrap();

    let hostname = match env::var("COMP_HOSTNAME") {
        Ok(val) => val,
        Err(_) => "0.0.0.0".to_string(),
    };
    let port = match env::var("COMP_PORT") {
        Ok(val) => val.parse::<u16>().unwrap(),
        Err(_) => 50051,
    };

    // TODO: Cleaner way to do this? This seems ridiculous.
    let addr = format!("{}:{:?}", hostname, port)
        .as_str()
        .to_socket_addrs()
        .unwrap()
        .next()
        .ok_or("failed to parse socket address")?;


    // Create the client.
    let hostname = match env::var("STUB_HOSTNAME") {
        Ok(val) => val,
        Err(_) => panic!("STUB_HOSTNAME not set. Can't connect to stub server."),
    };
    let port = match env::var("STUB_PORT") {
        Ok(val) => val.parse::<u16>().unwrap(),
        Err(_) => 50051,
    };
    let server_addr = format!("http://{}:{:?}", hostname, port);
    debug!("client connecting to {:?} ...", server_addr);
    // Connect the client to the server. Retry if connections fails, e.g. the server is not online
    // yet.
    let client = loop {
        match SensorManagerClient::connect(server_addr.clone()).await {
            Ok(client) => {
                break client;
            }
            Err(error) => {
                warn!("Failed to connect to gRPC server. Retrying. {:?}", error);
                tokio::time::sleep(tokio::time::Duration::from_millis(5000)).await;
            }
        };
    };

    // let mut sensor = MotifSensorProvider {
    //     stub: Arc::new(tokio::sync::Mutex::new(client)),
    //     is_test_running: tokio::sync::Mutex::new(false),
    // };
    let sensor = MotifSensorProvider::new(Arc::new(tokio::sync::Mutex::new(client)));

    debug!("starting server on {:?} ...", addr);

    Server::builder()
        .add_service(SensorProviderServer::new(sensor))
        .serve(addr)
        .await?;

    Ok(())
}
