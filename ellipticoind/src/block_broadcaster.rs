// pub fn broadcast_block(
//     block: (Block, Vec<Transaction>),
//     miners: Vec<Miner>,
// ) -> BoxFuture<'static, ()> {
//     async move {
//         join_all(
//             miners
//                 .iter()
//                 .cloned()
//                 .filter(|miner| miner.address != verification_key())
//                 .map(|miner| post_block(miner.host, &block)),
//         )
//         .await;
//     }
//     .boxed()
// }
