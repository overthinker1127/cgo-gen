#pragma once

#define MacroMaxRetries 3
#define MacroTimeoutMs (2500)
#define MacroFeatureMask 0x0F
#define MacroRatio 0.5
#define MacroScale 1e-3f
#define MacroHexFloat 0x1.8p+1
#define MacroPackageName "cgo-gen"
#define MacroDisplayName ("cgo-gen macro constants")
#define MacroFullName "cgo" "-gen"
#define MacroUnicodeName "caf\u00e9"
#define MacroRawName R"(unsupported raw string macro)"
#define MacroUtf8Name u8"unsupported utf8 macro"
#define MacroFlag(value) ((value) << 1)
