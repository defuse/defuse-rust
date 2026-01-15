#!/bin/bash
# Wraps a raw HTML file with Askama template syntax
# Usage: wrap_template.sh <input_file> <output_file>

input="$1"
output="$2"

{
    echo '{% extends "base.html" %}'
    echo ''
    printf '%s' '{% block content %}'
    cat "$input"
    echo '{% endblock %}'
} > "$output"
