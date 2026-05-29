#include "a_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "B.h"
#include "A.h"

AHandle* cgowrap_A_new(void) {
    return reinterpret_cast<AHandle*>(new A());
}

void cgowrap_A_delete(AHandle* self) {
    delete reinterpret_cast<A*>(self);
}

BHandle* cgowrap_A_Child(AHandle* self) {
    return reinterpret_cast<BHandle*>(reinterpret_cast<A*>(self)->Child());
}

int cgowrap_A_ChildValue(const AHandle* self) {
    return reinterpret_cast<const A*>(self)->ChildValue();
}

void cgowrap_A_SetChildValue(AHandle* self, int value) {
    reinterpret_cast<A*>(self)->SetChildValue(value);
}
