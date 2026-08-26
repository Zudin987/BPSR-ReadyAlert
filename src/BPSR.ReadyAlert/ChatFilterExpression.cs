using System.Collections.Concurrent;
using System.Text.RegularExpressions;

namespace BPSR.ReadyAlert;

/// <summary>
/// Friendly filter syntax layered on top of .NET regular expressions.
/// Matching is always case-insensitive. New lines, OR/||, and a whitespace-delimited
/// single pipe ("foo | bar") are friendly OR separators. A compact regex pipe
/// ("foo|bar" or "(foo|bar)") remains normal regex alternation for advanced users.
/// AND/&& combines clauses that must all match.
/// </summary>
internal static class ChatFilterExpression
{
    private const int MaxExpressionLength = 4096;
    private const int MaxCachedExpressions = 256;

    private static readonly Regex OrSplitter = new(
        @"[\r\n]+|\s*(?:\|\||\bOR\b)\s*|\s+\|\s+",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled);

    private static readonly Regex AndSplitter = new(
        @"\s*(?:&&|\bAND\b)\s*",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled);

    private static readonly TimeSpan MatchTimeout = TimeSpan.FromMilliseconds(80);
    private const RegexOptions MatchOptions =
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled;

    private static readonly ConcurrentDictionary<string, CompiledExpression> Cache =
        new(StringComparer.Ordinal);

    // A syntactically valid catastrophic regex can otherwise consume one timeout for
    // every historical message during a redraw. Once an expression times out, fail it
    // closed for the rest of the session and show a friendly editor error if revisited.
    private static readonly ConcurrentDictionary<string, byte> TimedOutExpressions =
        new(StringComparer.Ordinal);

    private sealed class CompiledExpression
    {
        internal Regex[][] Groups { get; init; } = [];
        internal string Error { get; init; } = string.Empty;
        internal bool Valid => Error.Length == 0;
    }

    internal static bool IsMatch(string text, string? expression)
    {
        if (string.IsNullOrWhiteSpace(expression)) return true;
        if (expression.Length > MaxExpressionLength) return false;
        if (TimedOutExpressions.ContainsKey(expression)) return false;

        var compiled = GetOrCompile(expression);
        if (!compiled.Valid) return false;

        try
        {
            var source = text ?? string.Empty;
            foreach (var andGroup in compiled.Groups)
            {
                var all = true;
                foreach (var atom in andGroup)
                {
                    if (!atom.IsMatch(source))
                    {
                        all = false;
                        break;
                    }
                }
                if (all) return true;
            }
        }
        catch (RegexMatchTimeoutException)
        {
            if (TimedOutExpressions.TryAdd(expression, 0))
                AppLog.Write("chat: regex filter timed out and was disabled for this session");
            Cache.TryRemove(expression, out _);
        }

        return false;
    }

    internal static bool TryValidate(string? expression, out string error)
    {
        error = string.Empty;
        if (string.IsNullOrWhiteSpace(expression)) return true;
        if (expression.Length > MaxExpressionLength)
        {
            error = $"Filter is too long. Keep it under {MaxExpressionLength} characters.";
            return false;
        }
        if (TimedOutExpressions.ContainsKey(expression))
        {
            error = "This regex timed out while matching chat and is disabled for this session. Simplify or change it.";
            return false;
        }

        var compiled = GetOrCompile(expression);
        error = compiled.Error;
        return compiled.Valid;
    }

    private static CompiledExpression GetOrCompile(string expression)
    {
        // User-editable filters normally number in the single digits. Bound the
        // process cache so repeatedly typing temporary expressions can never make it
        // grow for the lifetime of a long-running ReadyAlert session.
        if (Cache.Count >= MaxCachedExpressions && !Cache.ContainsKey(expression))
            Cache.Clear();

        return Cache.GetOrAdd(expression, Compile);
    }

    private static CompiledExpression Compile(string expression)
    {
        var groups = new List<Regex[]>();
        try
        {
            foreach (var orGroup in SplitNonEmpty(OrSplitter, expression))
            {
                var atoms = new List<Regex>();
                foreach (var atom in SplitNonEmpty(AndSplitter, orGroup))
                    atoms.Add(new Regex(atom, MatchOptions, MatchTimeout));

                if (atoms.Count > 0)
                    groups.Add(atoms.ToArray());
            }
        }
        catch (ArgumentException ex)
        {
            return new CompiledExpression { Error = ex.Message };
        }

        if (groups.Count == 0)
            return new CompiledExpression { Error = "Enter at least one word or regular expression." };

        return new CompiledExpression { Groups = groups.ToArray() };
    }

    private static IEnumerable<string> SplitNonEmpty(Regex splitter, string value)
    {
        foreach (var part in splitter.Split(value))
        {
            var trimmed = part.Trim();
            if (trimmed.Length > 0)
                yield return trimmed;
        }
    }

    internal static int CachedExpressionCountForSelfTest => Cache.Count;

    internal static void ClearCacheForSelfTest()
    {
        Cache.Clear();
        TimedOutExpressions.Clear();
    }
}
