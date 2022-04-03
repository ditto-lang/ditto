export declare function Err<T0, T2>($0: T2): Result<T0, T2>;
export declare function Just<T0>($0: T0): Maybe<T0>;
export declare type Maybe<T0> = ["Just", T0] | ["Nothing"];
export declare const Nothing: Maybe<never>;
export declare function Ok<T0, T2>($0: T0): Result<T0, T2>;
export declare type Phantom<T0> = ["Phantom", number];
export declare function Phantom<T0>($0: number): Phantom<T0>;
export declare type Result<T0, T1> = ["Err", T2] | ["Ok", T0];
export declare type Triple<T0, T1, T2> = ["Triple", T0, T2, T4];
export declare function Triple<T0, T2, T4>(
  $0: T0,
  $1: T2,
  $2: T4,
): Triple<T0, T2, T4>;
export declare type Unit = ["Unit"];
export declare const Unit: Unit;
export declare type Unknown = any;
