#include "vector_2_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "vector2.hpp"

Vector2Handle* cgowrap_Vector2_new__void(void) {
    return reinterpret_cast<Vector2Handle*>(new Vector2());
}

Vector2Handle* cgowrap_Vector2_new__int_int(int x, int y) {
    return reinterpret_cast<Vector2Handle*>(new Vector2(x, y));
}

void cgowrap_Vector2_delete(Vector2Handle* self) {
    delete reinterpret_cast<Vector2*>(self);
}

int cgowrap_Vector2_X(const Vector2Handle* self) {
    return reinterpret_cast<const Vector2*>(self)->X();
}

int cgowrap_Vector2_Y(const Vector2Handle* self) {
    return reinterpret_cast<const Vector2*>(self)->Y();
}

bool cgowrap_Vector2_OperBool(const Vector2Handle* self) {
    return reinterpret_cast<const Vector2*>(self)->operator bool();
}

Vector2Handle* cgowrap_Vector2_OperPlus(const Vector2Handle* self, const Vector2Handle* rhs) {
    return reinterpret_cast<Vector2Handle*>(new Vector2(reinterpret_cast<const Vector2*>(self)->operator+(*reinterpret_cast<const Vector2*>(rhs))));
}

bool cgowrap_Vector2_OperEqual(const Vector2Handle* self, const Vector2Handle* rhs) {
    return reinterpret_cast<const Vector2*>(self)->operator==(*reinterpret_cast<const Vector2*>(rhs));
}

Vector2Handle* cgowrap_OperMinus(const Vector2Handle* lhs, const Vector2Handle* rhs) {
    return reinterpret_cast<Vector2Handle*>(new Vector2(operator-(*reinterpret_cast<const Vector2*>(lhs), *reinterpret_cast<const Vector2*>(rhs))));
}
