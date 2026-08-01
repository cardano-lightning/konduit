use proptest::prelude::*;

use crate::AikenFn;
use konduit_data::{Cheque, Constants, Datum, Redeemer, Stage, Unlocked};

proptest! {

    #[test]
    fn prop_constants_conforms(x: Constants) {
        assert!(AikenFn::from_shortcut("wire/constants").eval_true(&x));
    }

    #[test]
    fn prop_unlocked_conforms(x: Unlocked) {
        assert!(AikenFn::from_shortcut("wire/unlocked").eval_true(&x));
    }

    #[test]
    fn prop_cheque_conforms(x: Cheque) {
        assert!(AikenFn::from_shortcut("wire/cheque").eval_true(&x));
    }

    #[test]
    fn prop_stage_conforms(stage: Stage) {
        assert!(AikenFn::from_shortcut("wire/stage").eval_true(&stage));
    }

    #[test]
    fn prop_datum_conforms(datum: Datum) {
        assert!(AikenFn::from_shortcut("wire/datum").eval_true(&datum));
    }

    #[test]
    fn prop_redeemer_conforms(redeemer: Redeemer) {
        assert!(AikenFn::from_shortcut("wire/redeemer").eval_true(&redeemer));
    }
}
