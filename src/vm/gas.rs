use crate::vm::VM;
use metered_wasmi::TrapKind;

impl<'a> VM<'a> {
    pub fn use_gas(&mut self, amount: u32) -> Result<(), metered_wasmi::TrapKind> {
        if self.gas < amount {
            Err(TrapKind::OutOfGas)
        } else {
            self.gas = self.gas - amount;
            Ok(())
        }
    }
}
