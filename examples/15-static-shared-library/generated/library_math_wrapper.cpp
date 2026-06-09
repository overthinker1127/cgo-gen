#include "library_math_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "library_math.hpp"

library_mathLibraryMathHandle* cgowrap_library_math_LibraryMath_new(int base) {
    return reinterpret_cast<library_mathLibraryMathHandle*>(new library_math::LibraryMath(base));
}

void cgowrap_library_math_LibraryMath_delete(library_mathLibraryMathHandle* self) {
    delete reinterpret_cast<library_math::LibraryMath*>(self);
}

int cgowrap_library_math_LibraryMath_add_static(const library_mathLibraryMathHandle* self, int value) {
    return reinterpret_cast<const library_math::LibraryMath*>(self)->add_static(value);
}

int cgowrap_library_math_LibraryMath_multiply_shared(const library_mathLibraryMathHandle* self, int value) {
    return reinterpret_cast<const library_math::LibraryMath*>(self)->multiply_shared(value);
}

int cgowrap_library_math_shared_multiplier(int value) {
    return library_math::shared_multiplier(value);
}

int cgowrap_library_math_static_offset(int value) {
    return library_math::static_offset(value);
}
