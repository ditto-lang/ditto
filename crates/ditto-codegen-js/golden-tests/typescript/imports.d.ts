import * as Data$Stuff from "Data.Stuff";
import * as test_stuff$Data$Stuff from "test-stuff/Data.Stuff";
export declare type MyFive = [
  "MyFive",
  Data$Stuff.Five,
  test_stuff$Data$Stuff.Five,
];
export declare function MyFive(
  $0: Data$Stuff.Five,
  $1: test_stuff$Data$Stuff.Five,
): MyFive;
export declare const anotherFive: test_stuff$Data$Stuff.Five;
export declare const five: Data$Stuff.Five;
export declare const justOneMore: MyFive;
export declare const maybeFive: test_stuff$Data$Stuff.Maybe<Data$Stuff.Five>;
export declare const myFive: MyFive;
