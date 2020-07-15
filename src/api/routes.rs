use crate::api::State;
use crate::api::{addresses, blocks, memory, state, storage, transactions, API};
use tide::sse;
use tide::Request;

impl API {
    pub fn routes(&mut self) {
        self.app
            .at("/")
            .get(sse::endpoint(|req: Request<State>, sender| async move {
                let mut new_block_receiver = req.state().new_block_broacaster.clone();
                while let Some(event) = new_block_receiver.recv().await {
sender.send("block", base64::encode(&event), Some(&base64::encode(&event))    ).await?;
                    
                    // println!("sending");
                    // match sender
                    //     .send(
                    //         "block",
                    //         base64::encode(&event),
                    //         Some(&base64::encode(&event)),
                    //     )
                    //     .await {
                    //
                    // Err(_) => println!("oops"),
                    // _ => (),
                    // };
                }
                Ok(())
            }));

        self.app.at("/blocks").post(blocks::create);
        self.app.at("/blocks").get(blocks::index);
        self.app.at("/blocks/:block_hash").get(blocks::show);
        self.app
            .at("/transactions/:transaction_hash")
            .get(transactions::show);
        self.app.at("/transactions").post(transactions::create);
        self.app
            .at("/memory/:contract_owner/:contract_name/:key")
            .get(memory::show);
        self.app
            .at("/storage/:contract_owner/:contract_name/:key")
            .get(storage::show);
        self.app.at("/state").get(state::show);
        self.app.at("/addresses/:address").get(addresses::show);
    }
}
