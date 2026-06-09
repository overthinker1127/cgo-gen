#include "library_math.hpp"

namespace library_math {

LibraryMath::LibraryMath(int base) : base_(base) {}

int LibraryMath::add_static(int value) const {
    return static_offset(base_ + value);
}

int LibraryMath::multiply_shared(int value) const {
    return shared_multiplier(base_ + value);
}

int static_offset(int value) {
    return value + 7;
}

} // namespace library_math
