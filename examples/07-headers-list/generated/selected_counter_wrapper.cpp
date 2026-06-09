#include "selected_counter_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "selected_counter.hpp"

SelectedCounterHandle* cgowrap_SelectedCounter_new(void) {
    return reinterpret_cast<SelectedCounterHandle*>(new SelectedCounter());
}

void cgowrap_SelectedCounter_delete(SelectedCounterHandle* self) {
    delete reinterpret_cast<SelectedCounter*>(self);
}

int cgowrap_SelectedCounter_Increment(const SelectedCounterHandle* self, int value) {
    return reinterpret_cast<const SelectedCounter*>(self)->Increment(value);
}
