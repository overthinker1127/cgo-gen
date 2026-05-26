#include "string_tool_wrapper.h"
#include <cstdlib>
#include <cstring>
#include <new>
#include <string>

#include "string_tool.hpp"

StringToolHandle* cgowrap_StringTool_new__void(void) {
    return reinterpret_cast<StringToolHandle*>(new StringTool());
}

StringToolHandle* cgowrap_StringTool_new__c_str(const char* prefix) {
    return reinterpret_cast<StringToolHandle*>(new StringTool(prefix));
}

void cgowrap_StringTool_delete(StringToolHandle* self) {
    delete reinterpret_cast<StringTool*>(self);
}

const char* cgowrap_StringTool_Prefix(const StringToolHandle* self) {
    return reinterpret_cast<const StringTool*>(self)->Prefix();
}

void cgowrap_StringTool_SetPrefix(StringToolHandle* self, const char* prefix) {
    reinterpret_cast<StringTool*>(self)->SetPrefix(prefix);
}

char* cgowrap_StringTool_Join(const StringToolHandle* self, char* value) {
    std::string result = reinterpret_cast<const StringTool*>(self)->Join(std::string(value != nullptr ? value : ""));
    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));
    if (buffer == nullptr) {
        return nullptr;
    }
    std::memcpy(buffer, result.c_str(), result.size() + 1);
    return buffer;
}

char* cgowrap_StringTool_EchoView(const StringToolHandle* self, char* value) {
    std::string result = reinterpret_cast<const StringTool*>(self)->EchoView(std::string(value != nullptr ? value : ""));
    char* buffer = static_cast<char*>(std::malloc(result.size() + 1));
    if (buffer == nullptr) {
        return nullptr;
    }
    std::memcpy(buffer, result.c_str(), result.size() + 1);
    return buffer;
}

void cgowrap_string_free(char* value) {
    free(value);
}
