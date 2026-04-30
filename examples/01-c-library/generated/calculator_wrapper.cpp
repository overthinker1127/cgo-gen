#include "calculator_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "calculator.h"

int cgowrap_calculator_add(int lhs, int rhs) {
    return calculator_add(lhs, rhs);
}

int cgowrap_calculator_scale(int value, int factor) {
    return calculator_scale(value, factor);
}

int cgowrap_calculator_subtract(int lhs, int rhs) {
    return calculator_subtract(lhs, rhs);
}
