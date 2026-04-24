#pragma once

#include <string.h>
#include "types.h"
#include "utils.h"
#include "data.h"

class DataRecord
{
public:
    DataRecord(void);
    ~DataRecord(void);

    inline uint32 GetId(void)        { return mData.nId; }
    inline uint32 GetTenantId(void)  { return mData.nTenantId; }
    inline uint32 GetNodeId(void)    { return mData.nNodeId; }
    inline NPCSTR GetName(void)      { return mData.sName; }
    inline NPCSTR GetCode(void)      { return mData.sCode; }
    inline uint16 GetSlot1_Act(void) { return mData.nSlot1_Act; }
    inline NPCSTR GetSlot1_Val(void) { return mData.sSlot1_Val; }
    inline uint16 GetSlot2_Act(void) { return mData.nSlot2_Act; }
    inline NPCSTR GetSlot2_Val(void) { return mData.sSlot2_Val; }
    inline uint16 GetSlot3_Act(void) { return mData.nSlot3_Act; }
    inline NPCSTR GetSlot3_Val(void) { return mData.sSlot3_Val; }

    inline void SetId(uint32 nId)              { mData.nId = nId; }
    inline void SetTenantId(uint32 nTenantId)  { mData.nTenantId = nTenantId; }
    inline void SetNodeId(uint32 nNodeId)      { mData.nNodeId = nNodeId; }
    inline void SetName(NPCSTR sName)          { strCopy(mData.sName, sName); }
    inline void SetCode(NPCSTR sCode)          { strCopy(mData.sCode, sCode); }
    inline void SetSlot1_Act(uint16 nAct)      { mData.nSlot1_Act = nAct; }
    inline void SetSlot1_Val(NPCSTR sVal)      { strCopy(mData.sSlot1_Val, sVal); }
    inline void SetSlot2_Act(uint16 nAct)      { mData.nSlot2_Act = nAct; }
    inline void SetSlot2_Val(NPCSTR sVal)      { strCopy(mData.sSlot2_Val, sVal); }
    inline void SetSlot3_Act(uint16 nAct)      { mData.nSlot3_Act = nAct; }
    inline void SetSlot3_Val(NPCSTR sVal)      { strCopy(mData.sSlot3_Val, sVal); }

private:
    TB_DATA_RECORD mData;
};
