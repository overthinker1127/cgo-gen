#ifndef CGOWRAP_CALCULATOR_WRAPPER_H
#define CGOWRAP_CALCULATOR_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

int cgowrap_calculator_add(int lhs, int rhs);
int cgowrap_calculator_scale(int value, int factor);
int cgowrap_calculator_subtract(int lhs, int rhs);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_CALCULATOR_WRAPPER_H */
