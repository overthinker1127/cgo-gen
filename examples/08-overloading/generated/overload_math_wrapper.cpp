#include "overload_math_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "overload_math.hpp"

OverloadMathHandle* cgowrap_OverloadMath_new__void(void) {
    return reinterpret_cast<OverloadMathHandle*>(new OverloadMath());
}

OverloadMathHandle* cgowrap_OverloadMath_new__int(int base) {
    return reinterpret_cast<OverloadMathHandle*>(new OverloadMath(base));
}

void cgowrap_OverloadMath_delete(OverloadMathHandle* self) {
    delete reinterpret_cast<OverloadMath*>(self);
}

double cgowrap_OverloadMath_Add__int_int_mut(OverloadMathHandle* self, int lhs, int rhs) {
    return reinterpret_cast<OverloadMath*>(self)->Add(lhs, rhs);
}

double cgowrap_OverloadMath_Add__double_double_mut(OverloadMathHandle* self, double lhs, double rhs) {
    return reinterpret_cast<OverloadMath*>(self)->Add(lhs, rhs);
}
