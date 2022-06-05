use super::GMFuncMarker;
use crate::{errors::Error, shared_states::GameSharedEntity, Result};
use serde_json::Value;

pub trait Tgm {
    fn handler_gm_cmd(&mut self, cmdstr: String) -> Result<()>;
}

impl Tgm for GameSharedEntity {
    // :TODO: 把 gm 指令函数名(func)独立出来,不需要再cmdstr里结构一遍了
    fn handler_gm_cmd(&mut self, cmdstr: String) -> Result<()> {
        match serde_json::from_str::<Value>(&cmdstr) {
            Ok(value) => match value {
                Value::Object(args) => match args.get("func") {
                    Some(func_name) => {
                        let func_name = func_name.as_str();
                        let func = if func_name.is_none() {
                            return Err(Error::from("[handler_gm_cmd]: func name error"));
                        } else {
                            let func_name = func_name.unwrap();
                            let func = GMFuncMarker::from_str(func_name).into_func();
                            if func.is_none() {
                                return Err(Error::from(format!(
                                    "[handler_gm_cmd]: {} not found",
                                    func_name
                                )));
                            }
                            func.unwrap()
                        };
                        func(self, Value::Object(args))
                    }
                    _ => Err(Error::from("[handler_gm_cmd]: func_name format error")),
                },
                _ => Err(Error::from(cmdstr)),
            },
            Err(err) => Err(Error::from(err.to_string())),
        }
    }
}
