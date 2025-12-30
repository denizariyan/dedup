from __future__ import annotations

import json
import re
import shutil
from abc import ABC, abstractmethod
from pathlib import Path
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from typing import ClassVar


class DedupTool(ABC):
    """Base class for duplicate-finding tools."""

    name: ClassVar[str]

    @abstractmethod
    def command(self, path: str) -> list[str]:
        pass

    def is_available(self) -> bool:
        return True

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        return None


class Dedup(DedupTool):
    name = "dedup"

    def _find_binary(self) -> str | None:
        if shutil.which("dedup"):
            return "dedup"
        script_dir = Path(__file__).parent.parent
        binary = script_dir / "target" / "release" / "dedup"
        if binary.exists():
            return str(binary)
        return None

    def command(self, path: str) -> list[str]:
        binary = self._find_binary()
        if binary is None:
            raise FileNotFoundError("dedup binary not found")
        return [binary, path, "--no-progress", "-f", "json"]

    def is_available(self) -> bool:
        return self._find_binary() is not None

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        try:
            data = json.loads(stdout)
            return data.get("stats", {}).get("duplicate_files", None)
        except (json.JSONDecodeError, KeyError):
            return None


class BashMd5(DedupTool):
    name = "bash+md5"

    def command(self, path: str) -> list[str]:
        cmd = f"find '{path}' -type f -exec md5sum {{}} + | awk '{{print $1}}' | sort | uniq -c | awk '$1 > 1 {{sum += $1}} END {{print sum+0}}'"
        return ["bash", "-c", cmd]

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        try:
            return int(stdout.strip())
        except ValueError:
            return None


class Fdupes(DedupTool):
    name = "fdupes"

    def command(self, path: str) -> list[str]:
        return ["fdupes", "-rq", path]

    def is_available(self) -> bool:
        return shutil.which("fdupes") is not None

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        lines = [line for line in stdout.strip().split("\n") if line.strip()]
        return len(lines) if lines else 0


class Fclones(DedupTool):
    name = "fclones"

    def command(self, path: str) -> list[str]:
        return ["fclones", "group", path]

    def is_available(self) -> bool:
        return shutil.which("fclones") is not None

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        count = 0
        for line in stdout.strip().split("\n"):
            line = line.strip()
            if line and (line.startswith("/") or line.startswith(".")):
                count += 1
        return count


class Rdfind(DedupTool):
    name = "rdfind"

    def command(self, path: str) -> list[str]:
        return [
            "rdfind",
            "-dryrun",
            "true",
            "-outputname",
            "/dev/null",
            path,
        ]

    def is_available(self) -> bool:
        return shutil.which("rdfind") is not None

    def parse_output(self, stdout: str, stderr: str) -> int | None:
        for pattern in [
            r"(\d+)\s+duplicate",
            r"Totally\s+(\d+)\s+files",
            r"It seems like you have\s+(\d+)",
        ]:
            match = re.search(pattern, stdout, re.IGNORECASE)
            if match:
                return int(match.group(1))
        for pattern in [r"(\d+)\s+duplicate"]:
            match = re.search(pattern, stderr, re.IGNORECASE)
            if match:
                return int(match.group(1))
        return None


ALL_TOOLS: list[type[DedupTool]] = [Dedup, Fclones, Fdupes, Rdfind, BashMd5]
