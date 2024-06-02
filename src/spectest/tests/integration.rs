use spectest;

struct MevalHandler<'a> {
    ctx: meval::Context<'a>,
}

impl<'a> MevalHandler<'a> {
    fn new() -> Self {
        Self {
            ctx: meval::Context::new(),
        }
    }
}

impl<'a> spectest::Handler for MevalHandler<'a> {
    type Error = String;

    fn enter(&mut self, background: &spectest::Background) -> Result<(), Self::Error> {
        for (var_name, var_value) in background.given.iter() {
            match var_value.trim().parse::<f64>() {
                Ok(var_value) => {
                    self.ctx.var(*var_name, var_value);
                }
                Err(err) => {
                    let msg = format!("cannot parse `{var_value}` as f64: {err}");
                    return Err(msg);
                }
            }
        }
        Ok(())
    }

    fn leave(&mut self, _background: &spectest::Background) -> Result<(), Self::Error> {
        self.ctx = meval::Context::new();
        Ok(())
    }

    fn example(&mut self, example: &mut spectest::Example) -> Result<(), Self::Error> {
        let Some(input) = example.when.get("input") else {
            let msg = format!("missing `input` definition in the 'When' spec");
            return Err(msg);
        };
        let input = match input.parse::<meval::Expr>() {
            Ok(expr) => expr,
            Err(err) => {
                let msg = format!("cannot parse `input` expression `{input}`: {err}");
                return Err(msg);
            }
        };

        match input.eval_with_context(self.ctx.clone()) {
            Ok(value) => {
                example.then.insert("result", value.to_string() + "\n");
            }
            Err(err) => {
                let msg = format!("cannot evaluate expression: {err}\n");
                example.then.insert("result", msg);
            }
        }

        Ok(())
    }
}

#[spectest::glob_test("testdata/integration/**/*.md")]
fn test(path: &str) {
    let mut handler = MevalHandler::new();
    spectest::run(path, &mut handler);
}
