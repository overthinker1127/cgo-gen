#pragma once

namespace library_math {

class LibraryMath {
public:
    explicit LibraryMath(int base);
    int add_static(int value) const;
    int multiply_shared(int value) const;

private:
    int base_;
};

int static_offset(int value);
int shared_multiplier(int value);

} // namespace library_math
