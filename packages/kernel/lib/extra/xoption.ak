pub fn min(l: Option<Int>, r: Int) {
  when l is {
    None -> Some(r)
    Some(li) ->
      if r < li {
        Some(r)
      } else {
        l
      }
  }
}

pub fn max(l: Option<Int>, r: Int) {
  when l is {
    None -> Some(r)
    Some(li) ->
      if r > li {
        Some(r)
      } else {
        l
      }
  }
}

pub fn min2(l: Option<Int>, r: Option<Int>) {
  when r is {
    Some(ri) -> min(l, ri)
    None -> l
  }
}

pub fn max2(l: Option<Int>, r: Option<Int>) {
  when r is {
    Some(ri) -> max(l, ri)
    None -> l
  }
}
