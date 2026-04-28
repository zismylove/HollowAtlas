from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .core import PackConfig, scan_folder
from .core.packer import pack_folder
from .core.types import OutputFormat, SplitMode


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)

    if args.command == "scan":
        return run_scan(args)
    if args.command == "pack":
        return run_pack(args)
    if args.command == "gui":
        from .gui.app import run_app

        run_app()
        return 0

    parser.print_help()
    return 1


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="hollowatlas", description="Texture atlas packer with tileset-friendly grid workflows.")
    subparsers = parser.add_subparsers(dest="command")

    scan = subparsers.add_parser("scan", help="Scan an asset folder.")
    scan.add_argument("input", help="Input asset folder.")
    scan.add_argument("--json", action="store_true", help="Print full scan JSON.")

    pack = subparsers.add_parser("pack", help="Pack an asset folder into atlas PNG + .tpsheet.")
    pack.add_argument("input", help="Input asset folder.")
    pack.add_argument("output", help="Output folder.")
    pack.add_argument("--max-size", type=int, default=2048, choices=[512, 1024, 2048, 4096, 8192])
    pack.add_argument("--padding", type=int, default=2, choices=[0, 1, 2, 4, 8])
    pack.add_argument("--extrude", type=int, default=1, choices=[0, 1, 2, 4])
    pack.add_argument("--no-trim", action="store_true", help="Disable transparent trim.")
    pack.add_argument("--allow-rotation", action="store_true", help="Allow 90 degree rotation while packing.")
    pack.add_argument("--no-power-of-two", action="store_true", help="Disable power-of-two atlas dimensions.")
    pack.add_argument("--no-square", action="store_true", help="Disable forced square atlas dimensions.")
    pack.add_argument(
        "--split-mode",
        default=SplitMode.ALL_IN_ONE.value,
        choices=[mode.value for mode in SplitMode],
    )
    pack.add_argument(
        "--output-format",
        default=OutputFormat.GODOT_TPSHEET.value,
        choices=[fmt.value for fmt in OutputFormat],
    )
    pack.add_argument("--debug-json", action="store_true", help="Also write atlas_N.debug.json.")

    subparsers.add_parser("gui", help="Open the Tkinter desktop GUI.")
    return parser


def run_scan(args: argparse.Namespace) -> int:
    result = scan_folder(args.input)
    if args.json:
        print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2))
    else:
        print(f"Input: {Path(args.input).resolve()}")
        print(f"Images: {result.total_images}")
        for warning in result.warnings:
            print(f"warning: {warning}", file=sys.stderr)
    return 0


def run_pack(args: argparse.Namespace) -> int:
    config = PackConfig(
        max_size=args.max_size,
        padding=args.padding,
        extrude=args.extrude,
        trim=not args.no_trim,
        allow_rotation=args.allow_rotation,
        power_of_two=not args.no_power_of_two,
        square=not args.no_square,
        split_mode=SplitMode(args.split_mode),
        output_format=OutputFormat(args.output_format),
        debug_json=args.debug_json or args.output_format == OutputFormat.JSON_DEBUG.value,
    )
    result = pack_folder(args.input, args.output, config)
    for log in result.logs:
        print(f"[{log.level}] {log.message}")
    print(json.dumps(result.to_dict(), ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
