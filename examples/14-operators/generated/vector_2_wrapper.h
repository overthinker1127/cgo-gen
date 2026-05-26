#ifndef CGOWRAP_VECTOR_2_WRAPPER_H
#define CGOWRAP_VECTOR_2_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Vector2Handle Vector2Handle;

Vector2Handle* cgowrap_Vector2_new__void(void);
Vector2Handle* cgowrap_Vector2_new__int_int(int x, int y);
void cgowrap_Vector2_delete(Vector2Handle* self);
int cgowrap_Vector2_X(const Vector2Handle* self);
int cgowrap_Vector2_Y(const Vector2Handle* self);
bool cgowrap_Vector2_OperBool(const Vector2Handle* self);
Vector2Handle* cgowrap_Vector2_OperPlus(const Vector2Handle* self, const Vector2Handle* rhs);
bool cgowrap_Vector2_OperEqual(const Vector2Handle* self, const Vector2Handle* rhs);
Vector2Handle* cgowrap_OperMinus(const Vector2Handle* lhs, const Vector2Handle* rhs);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_VECTOR_2_WRAPPER_H */
