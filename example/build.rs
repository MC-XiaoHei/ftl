use ftl_codegen::BuiltInFuncDef;

fn main() {
    println!("cargo:rerun-if-changed=locales");

    ftl_codegen::generator()
        .module_path("i18n")
        .register(test_builtin_func())
        .generate();
}

fn test_builtin_func() -> BuiltInFuncDef {
    ftl_codegen::ftl_builtin! {
        Test(FluentNum) {
            operator: String,
            operand: f64,
        }

        impl |this, out| {
            let v = *this.value;
            let op = this.operator.as_deref().unwrap_or("+");
            let operand = this.operand.unwrap_or(0.0);
            let result = match op {
                "+" => v + operand,
                "-" => v - operand,
                "x" | "*" => v * operand,
                "/" => if operand != 0.0 { v / operand } else { f64::NAN },
                _ => v,
            };
            write!(out, "{}", result).unwrap();
        }
    }
}
