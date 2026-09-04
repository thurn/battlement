# 19. InfoBadge and Privacy Policy

[Plan and shared contracts](../../chess-ui-implementation-plan.md#reading-guide)

"InfoBadge opens accessible crash-report help and activates Privacy Policy;
data erasure remains absent."

**Visible result.** Upload Crash Reports appears checked with the source
small circular blue “i” badge beside its label. Activating the badge opens the
source help dialog: no invented visible title, the body “We upload crash
reports to Unity Diagnostics.”, the cyan underlined Privacy Policy link, and
OK. Match badge and link geometry, typography, border, glow, and placement.

**Exercise.** Open help without toggling the checkbox; activate the link
through a test host; dismiss and restore badge focus. Reset returns to the
checked row with help closed. Host rejection keeps the same dialog visible.

**Deferred.** Modal animation is Task 28; full Gameplay composition Task 35.
The help body is not shown permanently beside the checkbox, and no erase
interaction belongs to this page.
