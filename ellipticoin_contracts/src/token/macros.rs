#[macro_export]
macro_rules! pay {
    ($db:expr, $recipient:expr, $token:expr, $amount:expr) => {
        Token::transfer($db, Self::address(), $recipient, $amount, $token)
    };
}

#[macro_export]
macro_rules! charge {
    ($db:expr, $sender:expr, $token:expr, $amount:expr) => {
        Token::transfer($db, $sender, Self::address(), $amount, $token)
    };
}
