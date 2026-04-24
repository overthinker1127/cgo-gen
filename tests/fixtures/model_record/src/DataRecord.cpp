#include "DataRecord.h"

DataRecord::DataRecord(void)
{
    memset(&mData, 0x00, sizeof(mData));
}

DataRecord::~DataRecord(void)
{
}
