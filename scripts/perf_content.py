#!/usr/bin/env python3

"""Sanitize performance-source content before it enters a report."""

from __future__ import annotations

import base64
import binascii
import hashlib
from typing import Any


def sanitize(value: Any) -> Any:
    """Preserve text and structured data while replacing unsafe large blobs."""
    if isinstance(value, str) and value.startswith("data:"):
        return {"data_url_metadata": _data_url_metadata(value)}
    if isinstance(value, list):
        return [sanitize(item) for item in value]
    if not isinstance(value, dict):
        return value
    result = {}
    for key, child in value.items():
        if key == "encrypted_content":
            continue
        if key in {"data", "blob"} and isinstance(child, str):
            encoded = child.encode(errors="replace")
            result[f"{key}_metadata"] = {
                "size": len(encoded),
                "sha256": hashlib.sha256(encoded).hexdigest(),
            }
            continue
        result[key] = sanitize(child)
    return result


def _data_url_metadata(value: str) -> dict[str, Any]:
    header, separator, payload = value.partition(",")
    mime_type = header[5:].partition(";")[0] or "text/plain"
    encoded = payload.encode(errors="replace") if separator else value.encode(errors="replace")
    decoded = encoded
    encoding = "text"
    if separator and ";base64" in header.casefold():
        try:
            decoded = base64.b64decode(encoded, validate=True)
            encoding = "base64"
        except (binascii.Error, ValueError):
            encoding = "invalid-base64"
    return {
        "mime_type": mime_type,
        "encoding": encoding,
        "size": len(decoded),
        "sha256": hashlib.sha256(decoded).hexdigest(),
    }


def flatten_text(value: Any) -> str:
    """Return searchable text from nested tool payloads."""
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return "\n".join(flatten_text(item) for item in value)
    if isinstance(value, dict):
        return "\n".join(flatten_text(item) for item in value.values())
    return ""
