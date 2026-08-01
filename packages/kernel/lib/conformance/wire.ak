use konduit/types as t

pub fn constants(x: t.Constants) -> Bool {
  let x: Data = x
  expect _: t.Constants = x
  True
}

pub fn unlocked(x: t.Unlocked) -> Bool {
  let x: Data = x
  expect _: t.Unlocked = x
  True
}

pub fn cheque(x: t.Cheque) -> Bool {
  let x: Data = x
  expect _: t.Cheque = x
  True
}

pub fn squash(x: t.Squash) -> Bool {
  let x: Data = x
  expect _: t.Squash = x
  True
}

pub fn datum(x: t.Datum) -> Bool {
  let x: Data = x
  expect _: t.Datum = x
  True
}

pub fn redeemer(x: t.Redeemer) -> Bool {
  let x: Data = x
  expect _: t.Redeemer = x
  True
}

pub fn stage(x: t.Stage) -> Bool {
  let x: Data = x
  expect _: t.Stage = x
  True
}

pub fn step(x: t.Step) -> Bool {
  let x: Data = x
  expect _: t.Step = x
  True
}
