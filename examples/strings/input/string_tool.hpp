#pragma once

#include <string>
#include <string_view>

class StringTool {
public:
    StringTool()
        : prefix_("item") {}

    explicit StringTool(const char* prefix)
        : prefix_(prefix != nullptr ? prefix : "") {}

    ~StringTool() = default;

    const char* Prefix() const {
        return prefix_.c_str();
    }

    void SetPrefix(const char* prefix) {
        prefix_ = prefix != nullptr ? prefix : "";
    }

    std::string Join(std::string value) const {
        return prefix_ + ":" + value;
    }

    std::string EchoView(std::string_view value) const {
        return std::string(value);
    }

private:
    std::string prefix_;
};
