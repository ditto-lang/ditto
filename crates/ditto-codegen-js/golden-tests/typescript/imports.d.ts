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
export declare const another_five: test_stuff$Data$Stuff.Five;
export declare const five: Data$Stuff.Five;
export declare const just_one_more: MyFive;
export declare const maybe_five: test_stuff$Data$Stuff.Maybe<Data$Stuff.Five>;
export declare const my_five: MyFive;
