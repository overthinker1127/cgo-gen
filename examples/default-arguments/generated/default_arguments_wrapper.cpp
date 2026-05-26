#include "default_arguments_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "default_arguments.hpp"

DefaultCounterHandle* cgowrap_DefaultCounter_new__int(int start) {
    return reinterpret_cast<DefaultCounterHandle*>(new DefaultCounter(start));
}

DefaultCounterHandle* cgowrap_DefaultCounter_new__void(void) {
    return reinterpret_cast<DefaultCounterHandle*>(new DefaultCounter());
}

void cgowrap_DefaultCounter_delete(DefaultCounterHandle* self) {
    delete reinterpret_cast<DefaultCounter*>(self);
}

int cgowrap_DefaultCounter_Value(const DefaultCounterHandle* self) {
    return reinterpret_cast<const DefaultCounter*>(self)->Value();
}

int cgowrap_DefaultCounter_Add__int_int_mut(DefaultCounterHandle* self, int value, int multiplier) {
    return reinterpret_cast<DefaultCounter*>(self)->Add(value, multiplier);
}

int cgowrap_DefaultCounter_Add__int_mut(DefaultCounterHandle* self, int value) {
    return reinterpret_cast<DefaultCounter*>(self)->Add(value);
}

int cgowrap_Clamp__int_int(int value, int max) {
    return Clamp(value, max);
}

int cgowrap_Clamp__int(int value) {
    return Clamp(value);
}
