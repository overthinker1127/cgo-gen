#include "b_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "B.h"

BHandle* cgowrap_B_new(void) {
    return reinterpret_cast<BHandle*>(new B());
}

void cgowrap_B_delete(BHandle* self) {
    delete reinterpret_cast<B*>(self);
}

int cgowrap_B_Value(const BHandle* self) {
    return reinterpret_cast<const B*>(self)->Value();
}

void cgowrap_B_SetValue(BHandle* self, int value) {
    reinterpret_cast<B*>(self)->SetValue(value);
}
