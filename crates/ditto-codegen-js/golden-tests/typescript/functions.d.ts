export declare function $void<T0>($0: T0): undefined;
export declare function always<T0, T1>($0: T0): ($0: T1) => T0;
export declare function curry<T1, T2, T3>(
  $0: ($0: T1, $1: T2) => T3,
): ($0: T1) => ($0: T2) => T3;
export declare function lazy(): number;
export declare function uncurry<T1, T2, T4>(
  $0: ($0: T1) => ($0: T2) => T4,
): ($0: T1, $1: T2) => T4;
