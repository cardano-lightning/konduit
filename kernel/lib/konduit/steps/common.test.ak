use konduit/steps/common.{next_index}

test test_next_index_new() {
  // prev < curr: returns (curr, [])
  let index, exclude <- next_index(0, [2, 3], 5)
  index == 5 && exclude == []
}

test test_next_index_persist() {
  // prev >= curr: returns (prev, drop_hard(exclude, curr))
  // drop_hard([2,3], 3) = [] — returns tail AFTER the found element
  let index, exclude <- next_index(5, [2, 3], 3)
  index == 5 && exclude == []
}

test test_next_index_persist_with_tail() {
  // tail after curr is non-empty: drop_hard([2,3,7], 3) = [7]
  let index, exclude <- next_index(5, [2, 3, 7], 3)
  index == 5 && exclude == [7]
}

test test_next_index_equal() {
  // prev == curr treated as prev >= curr
  let index, exclude <- next_index(3, [2, 3, 7], 3)
  index == 3 && exclude == [7]
}

test test_next_index_missing_from_exclude() fail {
  // curr not in exclude: drop_hard should fail
  let index, _exclude <- next_index(5, [2, 4], 3)
  index == 5
}
