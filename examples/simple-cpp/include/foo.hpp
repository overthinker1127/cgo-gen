#pragma once
#include <string>

namespace foo {
class Bar {
public:
    explicit Bar(int value);
    ~Bar();

    int value() const;
    void set_value(int value);
    std::string name() const;

private:
    int value_;
};

int add(int lhs, int rhs);
} // namespace foo
