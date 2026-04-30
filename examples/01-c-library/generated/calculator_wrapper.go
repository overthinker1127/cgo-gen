package generated

/*
#include <stdlib.h>
#include "calculator_wrapper.h"
*/
import "C"

func CalculatorAdd(lhs int32, rhs int32) int32 {
    return int32(C.cgowrap_calculator_add(C.int(lhs), C.int(rhs)))
}

func CalculatorScale(value int32, factor int32) int32 {
    return int32(C.cgowrap_calculator_scale(C.int(value), C.int(factor)))
}

func CalculatorSubtract(lhs int32, rhs int32) int32 {
    return int32(C.cgowrap_calculator_subtract(C.int(lhs), C.int(rhs)))
}
