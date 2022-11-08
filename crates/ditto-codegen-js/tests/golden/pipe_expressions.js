function tagged_identity(a, tag) {
  return a;
}
const got_ = tagged_identity(
  tagged_identity(tagged_identity(5, "1"), "2"),
  "3",
);
const want = tagged_identity(
  tagged_identity(tagged_identity(5, "1"), "2"),
  "3",
);
const inline_function = (n => [4, n])(5);
const five = 5;
function identity(a) {
  return a;
}
const without_parens = identity(5);
const with_parens = identity(5);
export {
  five,
  got_,
  identity,
  inline_function,
  tagged_identity,
  want,
  with_parens,
  without_parens,
};
