"""Check that a release is a native x64 GUI EXE with only Windows DLL imports."""
import struct
from pathlib import Path

SYSTEM_DLLS = {
    "advapi32.dll", "avrt.dll", "bcrypt.dll", "bcryptprimitives.dll", "comctl32.dll",
    "comdlg32.dll", "d3d11.dll", "dwmapi.dll", "dxgi.dll", "gdi32.dll", "imm32.dll",
    "kernel32.dll", "msvcrt.dll", "ntdll.dll", "ole32.dll", "oleaut32.dll", "opengl32.dll",
    "propsys.dll", "rpcrt4.dll", "secur32.dll", "shell32.dll", "shlwapi.dll", "ucrtbase.dll",
    "uiautomationcore.dll", "user32.dll", "userenv.dll", "uxtheme.dll", "version.dll",
    "winmm.dll", "ws2_32.dll",
}


def verify_exe(path: Path, notices: Path) -> list[str]:
    data = path.read_bytes()
    if data[:2] != b"MZ":
        raise ValueError("Not a Windows executable")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("Invalid PE signature")
    machine, count = struct.unpack_from("<HH", data, pe + 4)
    optional = pe + 24
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    magic = struct.unpack_from("<H", data, optional)[0]
    subsystem = struct.unpack_from("<H", data, optional + 68)[0]
    if machine != 0x8664 or magic != 0x20B or subsystem != 2:
        raise ValueError("Expected a Windows x64 GUI release (no console window)")
    sections = []
    for index in range(count):
        section = optional + optional_size + index * 40
        virtual_size, address, raw_size, raw_offset = struct.unpack_from("<IIII", data, section + 8)
        sections.append((address, max(virtual_size, raw_size), raw_offset, raw_size))

    def offset(rva):
        for address, size, raw_offset, raw_size in sections:
            if address <= rva < address + size and rva - address < raw_size:
                return raw_offset + rva - address
        raise ValueError(f"Unmapped PE address: {rva:#x}")

    imports = []
    directory, size = struct.unpack_from("<II", data, optional + 112 + 8)
    if not directory or not size:
        raise ValueError("Executable has no Windows imports")
    entry = offset(directory)
    end = entry + size
    while entry + 20 <= end:
        descriptor = struct.unpack_from("<IIIII", data, entry)
        if not any(descriptor):
            break
        name = offset(descriptor[3])
        terminator = data.index(b"\0", name)
        dll = data[name:terminator].decode("ascii").lower()
        if dll not in SYSTEM_DLLS and not dll.startswith(("api-ms-win-", "ext-ms-win-")):
            raise ValueError(f"External runtime dependency: {dll}")
        imports.append(dll)
        entry += 20
    delayed, _ = struct.unpack_from("<II", data, optional + 112 + 13 * 8)
    if delayed:
        raise ValueError("Delay-loaded dependencies need explicit release review")
    if notices.read_bytes() not in data:
        raise ValueError("The executable is missing its embedded license archive")
    return sorted(set(imports))
