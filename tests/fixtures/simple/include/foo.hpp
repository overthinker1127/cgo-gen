#pragma once

#include <string>

namespace foo {

enum Mode {
    MODE_A = 0,
    MODE_B = 1,
};

class Bar {
public:
    Bar(int value);
    ~Bar();

    int value() const;
    void set_value(int value);
    std::string name() const;
};

int add(int lhs, int rhs);

} // namespace foo
