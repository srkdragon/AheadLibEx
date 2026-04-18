#pragma once

#include <windows.h>

namespace aheadlibex {

HMODULE proxy_module() noexcept;
HMODULE original_module() noexcept;
HANDLE stop_event() noexcept;
bool stop_requested() noexcept;

} // namespace aheadlibex

namespace aheadlibex::user {

struct patch_lifecycle {
    bool run_on_process_attach;
    bool create_worker_thread;
};

patch_lifecycle configure_patch() noexcept;
bool on_process_attach() noexcept;
bool on_worker_thread() noexcept;
void on_process_detach(bool process_terminating) noexcept;

} // namespace aheadlibex::user
