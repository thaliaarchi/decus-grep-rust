# DECUS grep in Rust

A port of the historical DECUS [grep.c](grep.c) to Rust.

grep.c was originally written in 1980 for VMS and distributed for free as source
by the Digital Equipment Corporation Users' Society. It was easy to embed and
appeared in in many applications, and was particularly influential in DOS
circles.

To modern eyes, it has peculiar syntax. Optional is written as `-` instead of
`?`. Named character classes are written with colon instead of backslash: `:a`
for `[A-Za-z]`, `:d` for `[0-9]`, `:n` for `[A-Za-z0-9]`, and `: ` for `[␁- ]`
(Perl `[\x01- ]`). The pattern and searched text are both case-insensitive.

More striking are the features it is missing. It has no alternation nor group,
so sub-patterns that are not exactly one byte cannot be alternated or repeated.

I have reconstructed an early version of DECUS grep in [./grep.c](grep.c) and
minimally updated it for modern compilers in [./grep_modern.c](grep_modern.c).

Its original documentation is as follows:

```
grep searches a file for a given pattern.  Execute by
grep [flags] regular_expression file_list

Flags are single characters preceeded by '-':
-c      Only a count of matching lines is printed
-f      Print file name for matching lines switch, see below
-n      Each line is preceeded by its line number
-v      Only print non-matching lines

The file_list is a list of files (wildcards are acceptable on RSX modes).

The file name is normally printed if there is a file given.
The -f flag reverses this action (print name no file, not if more).

The regular_expression defines the pattern to search for.  Upper- and
lower-case are always ignored.  Blank lines never match.  The expression
should be quoted to prevent file-name translation.
x      An ordinary character (not mentioned below) matches that character.
'\'    The backslash quotes any character.  "\$" matches a dollar-sign.
'^'    A circumflex at the beginning of an expression matches the
       beginning of a line.
'$'    A dollar-sign at the end of an expression matches the end of a line.
'.'    A period matches any character except "new-line".
':a'   A colon matches a class of characters described by the following
':d'     character.  ":a" matches any alphabetic, ":d" matches digits,
':n'     ":n" matches alphanumerics, ": " matches spaces, tabs, and
': '     other control characters, such as new-line.
'*'    An expression followed by an asterisk matches zero or more
       occurrances of that expression: "fo*" matches "f", "fo"
       "foo", etc.
'+'    An expression followed by a plus sign matches one or more
       occurrances of that expression: "fo+" matches "fo", etc.
'-'    An expression followed by a minus sign optionally matches
       the expression.
'[]'   A string enclosed in square brackets matches any character in
       that string, but no others.  If the first character in the
       string is a circumflex, the expression matches any character
       except "new-line" and the characters in the string.  For
       example, "[xyz]" matches "xx" and "zyx", while "[^xyz]"
       matches "abc" but not "axb".  A range of characters may be
       specified by two characters separated by "-".  Note that,
       [a-z] matches alphabetics, while [z-a] never matches.
The concatenation of regular expressions is a regular expression.
```
