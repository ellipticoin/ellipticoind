use graphql_client::*;

mod helpers;

type Bytes = String;
type U32 = String;
type U64 = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "dist/schema.graphql",
    query_path = "dist/post_block.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct PostBlock;

// pub async fn post_block(
//     host: String,
//     block: &(models::block::Block, Vec<models::transaction::Transaction>),
// ) {
// let signed_block = sign(block).await;
// let request_body = PostBlock::build_query(post_block::Variables {
//     block: base64_encode(signed_block),
// });
//
// let _ = surf::post(host_uri(&host))
//     .body(http_types::Body::from_json(&request_body).unwrap())
//     .await;
// }
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "dist/schema.graphql",
    query_path = "dist/post_transaction.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct PostTransaction;

// pub async fn post_transaction(host: &str, transaction_request: Transaction) -> Transaction {
// let signed_transaction = sign(transaction_request).await;
// let request_body = PostTransaction::build_query(post_transaction::Variables {
//     transaction: base64_encode(signed_transaction),
// });
//
// let mut res = surf::post(host_uri(&host))
//     .body(http_types::Body::from_json(&request_body).unwrap())
//     .await
//     .unwrap();
// let response_data: Response<post_transaction::ResponseData> = res.body_json().await.unwrap();
// response_data.data.unwrap().post_transaction.into()
//     Transaction {
//         ..Default::default()
//     }
// }

// impl From<post_transaction::PostTransactionPostTransaction> for Transaction {
//     fn from(transaction: post_transaction::PostTransactionPostTransaction) -> Self {
//         Self {
//             id: transaction.id.parse().unwrap_or(0),
//             network_id: transaction.network_id.parse().unwrap_or(0),
//             block_number: transaction.block_number.parse().unwrap_or(0),
//             position: transaction.position.parse().unwrap_or(0),
//             contract: transaction.contract,
//             sender: base64::decode(transaction.sender).unwrap_or(vec![]),
//             transaction_number: transaction.transaction_number.parse().unwrap_or(0),
//             function: transaction.function,
//             arguments: base64::decode(transaction.arguments).unwrap_or(vec![]),
//             return_value: base64::decode(transaction.return_value).unwrap_or(vec![]),
//             raw: base64::decode(transaction.raw).unwrap_or(vec![]),
//         }
//     }
// }

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "dist/schema.graphql",
    query_path = "dist/block.graphql",
    response_derives = "Debug",
    normalization = "rust"
)]
struct Block;

// pub async fn get_block(block_number: u32) -> Result<(models::block::Block, Vec<Transaction>), ()> {
//     let request_body = Block::build_query(block::Variables {
//         block_number: block_number.to_string(),
//     });
//     let mut res = surf::post(host_uri(&random_bootnode().host))
//         .body(http_types::Body::from_json(&request_body).unwrap())
//         .await
//         .unwrap();
//     let response_body: Response<block::ResponseData> = res.body_json().await.unwrap();
//     let block_response = response_body.data.and_then(|data| data.block).ok_or(())?;
//     Ok(block_response.into())
// }
// impl From<Response<block::ResponseData>> for models::block::Block {
//     fn from(block: Response<block::ResponseData>) -> Self {
//         Self {
//             number: block.data.unwrap().block.unwrap().number.parse().unwrap(),
//             sealed: true,
//             ..Default::default()
//         }
//     }
// }
//
// impl From<block::BlockBlock> for (models::block::Block, Vec<models::transaction::Transaction>) {
//     fn from(block: block::BlockBlock) -> Self {
//         (
//             models::block::Block {
//                 number: block.number.parse().unwrap(),
//                 sealed: block.sealed,
//                 ..Default::default()
//             },
//             block
//                 .transactions
//                 .iter()
//                 .map(models::transaction::Transaction::from)
//                 .collect::<Vec<models::transaction::Transaction>>(),
//         )
//     }
// }
// impl From<&block::BlockBlockTransactions> for models::transaction::Transaction {
//     fn from(transaction: &block::BlockBlockTransactions) -> Self {
//         models::transaction::Transaction {
//             id: transaction.id.parse().unwrap(),
//             network_id: transaction.network_id.parse().unwrap(),
//             block_number: transaction.block_number.parse().unwrap(),
//             position: transaction.position.parse().unwrap(),
//             contract: transaction.contract.clone(),
//             sender: base64::decode(&transaction.sender).unwrap(),
//             transaction_number: transaction.transaction_number.parse().unwrap(),
//             function: transaction.function.clone(),
//             arguments: base64::decode(&transaction.arguments).unwrap(),
//             return_value: base64::decode(&transaction.return_value).unwrap(),
//             raw: base64::decode(&transaction.raw).unwrap(),
//         }
//     }
// }
//
// pub async fn random_peer() {}
//
// pub async fn download(file_name: &str, path: PathBuf, expected_hash: [u8; 32]) {
//     let mut response = surf::get(format!(
//         "{}/{}/{}",
//         host_uri(&random_bootnode().host),
//         "static",
//         file_name
//     ))
//     .await
//     .unwrap();
//     const CHUNK_SIZE: usize = 1024;
//     let mut buf = [0_u8; CHUNK_SIZE];
//     let length = response.len().unwrap();
//     let mut body = response.take_body().into_reader();
//     let file = File::create(path).unwrap();
//     let mut buf_writer = BufWriter::new(file);
//     let mut hasher = Sha256::new();
//     let pb = ProgressBar::new(length as u64);
//     pb.set_style(
//         indicatif::ProgressStyle::default_bar()
//             .template("{msg}\n[{elapsed_precise}] [{bar}] {pos}/{len} ({percent}%)")
//             .progress_chars("=> "),
//     );
//     pb.set_message(&format!("Downloading {}", file_name));
//     for _ in (0..length).step_by(CHUNK_SIZE) {
//         pb.inc(CHUNK_SIZE as u64);
//         let bytes_read = body.read(&mut buf).await.unwrap();
//         hasher.update(&buf[0..bytes_read]);
//         buf_writer.write_all(&buf[0..bytes_read]).unwrap();
//     }
//     pb.finish();
//     println!("Downloaded {}", file_name);
//     let hash: [u8; 32] = hasher.finalize().into();
//     if hash != expected_hash {
//         panic!(
//             "Invalid hash of {}. Expected {} but got {}",
//             file_name,
//             hex::encode(expected_hash),
//             hex::encode(hash.to_vec())
//         );
//     }
// }
