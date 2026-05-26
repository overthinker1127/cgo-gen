#ifndef CGOWRAP_OVERLOAD_MATH_WRAPPER_H
#define CGOWRAP_OVERLOAD_MATH_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OverloadMathHandle OverloadMathHandle;

OverloadMathHandle* cgowrap_OverloadMath_new__void(void);
OverloadMathHandle* cgowrap_OverloadMath_new__int(int base);
void cgowrap_OverloadMath_delete(OverloadMathHandle* self);
double cgowrap_OverloadMath_Add__int_int_mut(OverloadMathHandle* self, int lhs, int rhs);
double cgowrap_OverloadMath_Add__double_double_mut(OverloadMathHandle* self, double lhs, double rhs);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_OVERLOAD_MATH_WRAPPER_H */
