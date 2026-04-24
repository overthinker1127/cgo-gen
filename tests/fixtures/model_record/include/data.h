#pragma once

#include "defs.h"
#include "types.h"

typedef struct
{
    uint32 nId;
    uint32 nTenantId;
    uint32 nNodeId;
    char   sName[REC_NAME_SIZE];
    char   sCode[REC_CODE_SIZE];
    uint16 nSlot1_Act;
    char   sSlot1_Val[REC_VAL_SIZE];
    uint16 nSlot2_Act;
    char   sSlot2_Val[REC_VAL_SIZE];
    uint16 nSlot3_Act;
    char   sSlot3_Val[REC_VAL_SIZE];
} TB_DATA_RECORD;
