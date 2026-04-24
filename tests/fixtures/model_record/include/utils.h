#pragma once

#include <cstring>
#include "types.h"

inline void strCopy(char* dest, NPCSTR src) {
    std::strcpy(dest, src != nullptr ? src : "");
}
