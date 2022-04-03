import {
  arrayMapImpl as foreign$arrayMapImpl,
  h as foreign$h,
} from "./foreign.js";
function Attr($0, $1) {
  return ["Attr", $0, $1];
}
const arrayMap = foreign$arrayMapImpl;
function span(attrs) {
  return foreign$h("span", attrs);
}
export { Attr, arrayMap, span };
