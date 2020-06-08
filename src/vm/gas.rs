use crate::vm::VM;
use metered_wasmi::TrapKind;

impl<'a> VM<'a> {
    pub fn use_gas(&mut self, amount: u32) -> Result<(), metered_wasmi::TrapKind> {
        if let Some(gas) = self.gas {
            if gas < amount {
                Err(TrapKind::OutOfGas)
            } else {
                Ok(self.gas = Some(gas - amount))
            }
        } else {
            Ok(())
        }
    }
}
