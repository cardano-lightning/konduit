use crate::{
    Cheque, ChequeBody, Constants, Cont, Datum, Duration, Eol, Indexes, Lock, Pending, Redeemer,
    Secret, Signature, Squash, SquashBody, Stage, Step, Tag, Unlocked, Unpend, Unverified, Used,
    VerifyingKey,
    aiken::{ToAikenLiteral, bytes_to_hex_lit, list_lit},
    locked::Locked,
};

// ---------------------------------------------------------------------------
// Primitive types
// ---------------------------------------------------------------------------

impl ToAikenLiteral for u64 {
    fn to_aiken_literal(&self) -> String {
        self.to_string()
    }
}

impl ToAikenLiteral for Duration {
    fn to_aiken_literal(&self) -> String {
        let millis = self.as_millis() as u64;
        millis.to_string()
    }
}

impl ToAikenLiteral for Tag {
    fn to_aiken_literal(&self) -> String {
        bytes_to_hex_lit(self.as_ref())
    }
}

impl ToAikenLiteral for VerifyingKey {
    fn to_aiken_literal(&self) -> String {
        bytes_to_hex_lit(self.as_ref())
    }
}

impl ToAikenLiteral for Signature {
    fn to_aiken_literal(&self) -> String {
        bytes_to_hex_lit(self.as_ref())
    }
}

impl ToAikenLiteral for Lock {
    fn to_aiken_literal(&self) -> String {
        bytes_to_hex_lit(&self.0)
    }
}

impl ToAikenLiteral for Secret {
    fn to_aiken_literal(&self) -> String {
        bytes_to_hex_lit(&self.0)
    }
}

impl ToAikenLiteral for Unpend {
    fn to_aiken_literal(&self) -> String {
        match self {
            Unpend::Continue => bytes_to_hex_lit(b""),
            Unpend::Expire => bytes_to_hex_lit(&[0]),
            Unpend::Unlock(arr) => bytes_to_hex_lit(arr),
        }
    }
}

// ---------------------------------------------------------------------------
// Indexes → List<Int>
// ---------------------------------------------------------------------------

impl ToAikenLiteral for Indexes {
    fn to_aiken_literal(&self) -> String {
        if self.0.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                self.0
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Tuple / list types  (encode as Plutus Lists)
// ---------------------------------------------------------------------------

impl ToAikenLiteral for Used {
    // Aiken: type Used = (Index, Amount)
    fn to_aiken_literal(&self) -> String {
        format!("({}, {})", self.index, self.amount)
    }
}

impl ToAikenLiteral for Pending {
    // Aiken: type Pending = (Amount, Timeout, Lock)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {}, {})",
            self.amount.to_aiken_literal(),
            self.timeout.to_aiken_literal(),
            self.lock.to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for ChequeBody<Lock> {
    // Aiken: type ChequeBody = (Index, Amount, Timeout, Lock)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {}, {}, {})",
            self.index().to_aiken_literal(),
            self.amount().to_aiken_literal(),
            self.timeout().to_aiken_literal(),
            self.latch().to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for Locked<Unverified> {
    // Aiken: type Locked = (ChequeBody, Signature)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {})",
            self.body().to_aiken_literal(),
            self.signature.to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for SquashBody {
    // Aiken: type SquashBody = (Amount, Index, Exclude)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {}, {})",
            self.amount().to_aiken_literal(),
            self.index().to_aiken_literal(),
            self.exclude().to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for Squash<Unverified> {
    // Aiken: type Squash = (SquashBody, Signature)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {})",
            self.body().to_aiken_literal(),
            self.signature().to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for Datum {
    // Aiken: type Datum = (ScriptHash, Constants, Stage)
    fn to_aiken_literal(&self) -> String {
        format!(
            "({}, {}, {})",
            bytes_to_hex_lit(&self.own_hash),
            self.constants.to_aiken_literal(),
            self.stage.to_aiken_literal()
        )
    }
}

// ---------------------------------------------------------------------------
// Named constructor types (encode as Plutus Constr)
// ---------------------------------------------------------------------------

impl ToAikenLiteral for Constants {
    // Aiken: type Constants { tag, add_vkey, sub_vkey, close_period }
    fn to_aiken_literal(&self) -> String {
        format!(
            "t.Constants {{ tag: {}, add_vkey: {}, sub_vkey: {}, close_period: {} }}",
            self.tag.to_aiken_literal(),
            self.add_vkey.to_aiken_literal(),
            self.sub_vkey.to_aiken_literal(),
            self.close_period.to_aiken_literal()
        )
    }
}

impl ToAikenLiteral for Stage {
    fn to_aiken_literal(&self) -> String {
        match self {
            Stage::Opened(amount, useds) => {
                format!(
                    "t.Opened({}, {})",
                    amount.to_aiken_literal(),
                    list_lit(useds)
                )
            }
            Stage::Closed(amount, useds, duration) => format!(
                "t.Closed({}, {}, {})",
                amount.to_aiken_literal(),
                list_lit(useds),
                duration.to_aiken_literal()
            ),
            Stage::Responded(amount, pendings) => {
                format!(
                    "t.Responded({}, {})",
                    amount.to_aiken_literal(),
                    list_lit(pendings)
                )
            }
        }
    }
}

impl ToAikenLiteral for Cheque<Unverified> {
    fn to_aiken_literal(&self) -> String {
        match self {
            Cheque::Locked(l) => {
                // t.LockedCheque(ChequeBody, Signature) — matches Rust encoding
                format!(
                    "t.LockedCheque({}, {})",
                    l.body().to_aiken_literal(),
                    l.signature.to_aiken_literal()
                )
            }
            Cheque::Unlocked(u) => {
                // KNOWN MISMATCH: Aiken expects t.UnlockedCheque(locked_body, sig, secret)
                // but Rust encodes as (body_with_secret_in_latch, sig).
                // Render the Aiken-expected form so the encoding test intentionally fails.
                let locked_body = u.body().locked();
                let secret = u.body().secret();
                format!(
                    "t.UnlockedCheque({}, {}, {})",
                    locked_body.to_aiken_literal(),
                    u.signature.to_aiken_literal(),
                    secret.to_aiken_literal()
                )
            }
        }
    }
}

impl ToAikenLiteral for Eol {
    fn to_aiken_literal(&self) -> String {
        match self {
            Eol::End => "t.End".to_string(),
            Eol::Elapse => "t.Elapse".to_string(),
        }
    }
}

impl ToAikenLiteral for Cont {
    fn to_aiken_literal(&self) -> String {
        match self {
            Cont::Add => "t.Add".to_string(),
            Cont::Sub(squash, unlockeds) => {
                format!(
                    "t.Sub({}, {})",
                    squash.to_aiken_literal(),
                    list_aiken_unlocked(unlockeds)
                )
            }
            Cont::Close => "t.Close".to_string(),
            Cont::Respond(squash, cheques) => {
                format!(
                    "t.Respond({}, {})",
                    squash.to_aiken_literal(),
                    list_lit(cheques)
                )
            }
            Cont::Unlock(unpends) => {
                format!("t.Unlock({})", list_lit(unpends))
            }
            Cont::Expire(unpends) => {
                format!("t.Expire({})", list_lit(unpends))
            }
        }
    }
}

impl ToAikenLiteral for Step {
    fn to_aiken_literal(&self) -> String {
        match self {
            // Aiken Step constructors are StepCont / StepEol (not Cont / Eol)
            Step::Cont(cont) => format!("t.StepCont({})", cont.to_aiken_literal()),
            Step::Eol(eol) => format!("t.StepEol({})", eol.to_aiken_literal()),
        }
    }
}

impl ToAikenLiteral for Redeemer {
    fn to_aiken_literal(&self) -> String {
        match self {
            Redeemer::Defer => "t.Defer".to_string(),
            Redeemer::Main(steps) => format!("t.Main({})", list_lit(steps)),
            Redeemer::Mutual => "t.Mutual".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Render a list of `Unlocked<Unverified>` values as an Aiken list literal.
/// `Unlocked` is not `ToAikenLiteral` because its Aiken form differs from its
/// Rust encoding — only `Cheque::Unlocked` exposes that form.  Here we need the
/// raw `(locked_body, sig, secret)` tuple for the `Sub` constructor.
fn list_aiken_unlocked(items: &[Unlocked<Unverified>]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let rendered: Vec<String> = items
        .iter()
        .map(|u| {
            let locked_body = u.body().locked();
            let secret = u.body().secret();
            format!(
                "({}, {}, {})",
                locked_body.to_aiken_literal(),
                u.signature.to_aiken_literal(),
                secret.to_aiken_literal()
            )
        })
        .collect();
    format!("[{}]", rendered.join(", "))
}
