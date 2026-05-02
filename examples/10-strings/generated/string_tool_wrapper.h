#ifndef CGOWRAP_STRING_TOOL_WRAPPER_H
#define CGOWRAP_STRING_TOOL_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct StringToolHandle StringToolHandle;

StringToolHandle* cgowrap_StringTool_new__void(void);
StringToolHandle* cgowrap_StringTool_new__c_str(const char* prefix);
void cgowrap_StringTool_delete(StringToolHandle* self);
const char* cgowrap_StringTool_Prefix(const StringToolHandle* self);
void cgowrap_StringTool_SetPrefix(StringToolHandle* self, const char* prefix);
char* cgowrap_StringTool_Join(const StringToolHandle* self, char* value);
char* cgowrap_StringTool_EchoView(const StringToolHandle* self, char* value);
void cgowrap_string_free(char* value);

#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_STRING_TOOL_WRAPPER_H */
