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
    private static readonly Regex OrSplitter = new(
        @"[\r\n]+|\s*(?:\|\||\bOR\b)\s*|\s+\|\s+",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled);

    private static readonly Regex AndSplitter = new(
        @"\s*(?:&&|\bAND\b)\s*",
        RegexOptions.IgnoreCase | RegexOptions.CultureInvariant | RegexOptions.Compiled);

    private static readonly TimeSpan MatchTimeout = TimeSpan.FromMilliseconds(80);
    private const RegexOptions MatchOptions = RegexOptions.IgnoreCase | RegexOptions.CultureInvariant;

    internal static bool IsMatch(string text, string? expression)
    {
        if (string.IsNullOrWhiteSpace(expression)) return true;

        try
        {
            foreach (var orGroup in SplitNonEmpty(OrSplitter, expression))
            {
                var all = true;
                foreach (var atom in SplitNonEmpty(AndSplitter, orGroup))
                {
                    if (!Regex.IsMatch(text ?? string.Empty, atom, MatchOptions, MatchTimeout))
                    {
                        all = false;
                        break;
                    }
                }

                if (all) return true;
            }
        }
        catch (ArgumentException)
        {
            return false;
        }
        catch (RegexMatchTimeoutException)
        {
            return false;
        }

        return false;
    }

    internal static bool TryValidate(string? expression, out string error)
    {
        error = string.Empty;
        if (string.IsNullOrWhiteSpace(expression)) return true;

        var foundAtom = false;
        foreach (var orGroup in SplitNonEmpty(OrSplitter, expression))
        {
            foreach (var atom in SplitNonEmpty(AndSplitter, orGroup))
            {
                foundAtom = true;
                try
                {
                    _ = new Regex(atom, MatchOptions, MatchTimeout);
                }
                catch (ArgumentException ex)
                {
                    error = $"Invalid regex '{atom}': {ex.Message}";
                    return false;
                }
            }
        }

        if (!foundAtom)
        {
            error = "Enter at least one word or regular expression.";
            return false;
        }

        return true;
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
}
