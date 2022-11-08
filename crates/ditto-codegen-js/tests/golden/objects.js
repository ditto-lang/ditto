function do_object() {
  return {};
}
function pure_object() {
  return {};
}
function mk_has_foo(a) {
  return { foo: a };
}
const foo = mk_has_foo(true)["foo"];
function get_foo(x) {
  return x["foo"];
}
const very_nested = { a: { b: { c: { d: [] } } } };
const d = very_nested["a"]["b"]["c"]["d"];
const just_object_things = { yep: true, huh: undefined, why: () => ({}) };
const empty_object = {};
export {
  d,
  do_object,
  empty_object,
  foo,
  get_foo,
  just_object_things,
  mk_has_foo,
  pure_object,
  very_nested,
};
