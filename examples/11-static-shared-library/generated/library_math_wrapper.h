#ifndef CGOWRAP_LIBRARY_MATH_WRAPPER_H
#define CGOWRAP_LIBRARY_MATH_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct library_mathLibraryMathHandle library_mathLibraryMathHandle;

library_mathLibraryMathHandle* cgowrap_library_math_LibraryMath_new(int base);
void cgowrap_library_math_LibraryMath_delete(library_mathLibraryMathHandle* self);
int cgowrap_library_math_LibraryMath_add_static(const library_mathLibraryMathHandle* self, int value);
int cgowrap_library_math_LibraryMath_multiply_shared(const library_mathLibraryMathHandle* self, int value);
int cgowrap_library_math_shared_multiplier(int value);
int cgowrap_library_math_static_offset(int value);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_LIBRARY_MATH_WRAPPER_H */
