use aiken/crypto.{VerificationKey}
use aiken/primitive/int
use extra/xlist.{drop_hard}
use konduit/prelude.{Amount, Index}
use konduit/types as t

pub fn next_index(
  prev: Index,
  exclude: t.Exclude,
  curr: Index,
  return: fn(Index, t.Exclude) -> result,
) -> result {
  if prev < curr {
    return(curr, [])
  } else {
    return(prev, drop_hard(exclude, curr))
  }
}
// pub fn reduce_useds(
//   index: Index,
//   amount: Amount,
//   useds_in: List<t.Used>,
//   useds_out: List<t.Used>,
//   return: fn(Amount, List<t.Used>, List<t.Used>) -> result,
// ) -> result {
//   when useds_in is {
//     [] -> {
//       expect [(index_out, amount_out), ..useds_out] = useds_out
//       expect index == index_out
//       expect amount == amount_out
//       return(amount, useds_in, useds_out)
//     }
//     [(index_in, amount_in), ..useds_in] -> {
//       when int.compare(index, index_in) is {
//         Less -> {
//           when useds_out is {
//             [] -> return(0, [], [])
//           }
//           return(amount, useds_in, useds_out)
//         }
//         Equal -> {
//           return(amount, useds_in, useds_out)
//         }
//         Greater -> {
//           return(amount, useds_in, useds_out)
//         }
//       }
//     }
//   }
// }
// 
// fn drop_useds(
//   index: Index,
//   amount: Amount,
//   useds: List<t.Used>,
//   return: fn(List<t.Used>) -> result,
// ) -> result {
//   expect [(index_, amount_), ..useds] = useds
//   expect index == index_
//   expect amount == amount_
//   return(useds)
// }
