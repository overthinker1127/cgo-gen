#include "foo.hpp"

namespace foo {

Bar::Bar(int value) : value_(value) {}

Bar::~Bar() = default;

int Bar::value() const { return value_; }

void Bar::set_value(int value) { value_ = value; }

std::string Bar::name() const { return "foo"; }

int add(int lhs, int rhs) { return lhs + rhs; }

} // namespace foo
