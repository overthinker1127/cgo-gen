#pragma once

namespace boom {

class Worker {
public:
    Worker();
    ~Worker();
    int maybe(bool fail);
};

int fail_if(bool fail);

} // namespace boom
