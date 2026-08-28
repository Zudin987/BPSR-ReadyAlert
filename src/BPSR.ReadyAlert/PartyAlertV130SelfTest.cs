using System.Collections.Concurrent;

namespace BPSR.ReadyAlert;

internal static class PartyAlertV130SelfTest
{
    internal static void Run()
    {
        TestProtocolIdentity();
        TestInviteAndRequestRouting();
        TestDuplicateSuppression();
        TestIndependentFromChatOverlay();
        TestCoreAudioRouting();
    }

    private static void TestProtocolIdentity()
    {
        Assert(PartyAlertCaptureBridge.ServiceIdForSelfTest == 966_773_353UL,
            "GrpcTeamNtf service id matches BPSR protocol metadata");
        Assert(PartyAlertCaptureBridge.ApplyJoinMethodForSelfTest == 0x05,
            "NotifyApplyJoin method is 0x05");
        Assert(PartyAlertCaptureBridge.InvitationMethodForSelfTest == 0x06,
            "NotifyInvitation method is 0x06");
    }

    private static void TestInviteAndRequestRouting()
    {
        var events = new ConcurrentQueue<AlertEvent>();
        PartyAlertCaptureBridge.Configure(events);
        PartyAlertCaptureBridge.ResetForSelfTest();

        Assert(PartyAlertCaptureBridge.TryHandle(
                PartyAlertCaptureBridge.ServiceIdForSelfTest,
                PartyAlertCaptureBridge.InvitationMethodForSelfTest,
                [0x0A, 0x01, 0x11]),
            "party invitation is recognized");
        Assert(events.TryDequeue(out var invite), "party invitation enqueues a core event");
        Assert(invite.Kind == "party-invite" && invite.Title == "BPSR Party Invite",
            "party invitation uses the dedicated core event kind/title");

        Assert(PartyAlertCaptureBridge.TryHandle(
                PartyAlertCaptureBridge.ServiceIdForSelfTest,
                PartyAlertCaptureBridge.ApplyJoinMethodForSelfTest,
                [0x0A, 0x01, 0x22]),
            "party join request is recognized");
        Assert(events.TryDequeue(out var request), "party join request enqueues a core event");
        Assert(request.Kind == "party-request" && request.Title == "BPSR Party Join Request",
            "party join request uses the dedicated core event kind/title");

        Assert(!PartyAlertCaptureBridge.TryHandle(
                PartyAlertCaptureBridge.ServiceIdForSelfTest,
                0x07,
                [0x01]),
            "other GrpcTeamNtf methods are not treated as party alerts");
        Assert(!PartyAlertCaptureBridge.TryHandle(123UL, 0x06, [0x01]),
            "same method id on another service is not treated as a party alert");
    }

    private static void TestDuplicateSuppression()
    {
        var events = new ConcurrentQueue<AlertEvent>();
        PartyAlertCaptureBridge.Configure(events);
        PartyAlertCaptureBridge.ResetForSelfTest();

        var payload = new byte[] { 0x0A, 0x02, 0xAA, 0xBB };
        Assert(PartyAlertCaptureBridge.TryHandle(
            PartyAlertCaptureBridge.ServiceIdForSelfTest,
            PartyAlertCaptureBridge.InvitationMethodForSelfTest,
            payload), "first invite is accepted");
        Assert(PartyAlertCaptureBridge.TryHandle(
            PartyAlertCaptureBridge.ServiceIdForSelfTest,
            PartyAlertCaptureBridge.InvitationMethodForSelfTest,
            payload), "duplicate invite remains a handled notify");
        Assert(events.Count == 1, "identical invite duplicate is suppressed");

        Assert(PartyAlertCaptureBridge.TryHandle(
            PartyAlertCaptureBridge.ServiceIdForSelfTest,
            PartyAlertCaptureBridge.InvitationMethodForSelfTest,
            [0x0A, 0x02, 0xAA, 0xBC]), "different invite payload is accepted immediately");
        Assert(events.Count == 2, "different invitation is not suppressed by the time window");
    }

    private static void TestIndependentFromChatOverlay()
    {
        var events = new ConcurrentQueue<AlertEvent>();
        PartyAlertCaptureBridge.Configure(events);
        PartyAlertCaptureBridge.ResetForSelfTest();
        ChatCaptureBridge.Enabled = false;

        Assert(ChatCaptureBridge.TryHandle(
                PartyAlertCaptureBridge.ServiceIdForSelfTest,
                PartyAlertCaptureBridge.ApplyJoinMethodForSelfTest,
                [0x0A, 0x01, 0x33]),
            "shared notify dispatcher handles party request while Chat Overlay is disabled");
        Assert(events.TryDequeue(out var request) && request.Kind == "party-request",
            "Chat Overlay disabled does not suppress the core party alert");
    }

    private static void TestCoreAudioRouting()
    {
        Assert(TrayApplicationContext.IsCoreSoundEventForSelfTest("party-invite"),
            "party invitations use ReadyAlert's core alert audio path");
        Assert(TrayApplicationContext.IsCoreSoundEventForSelfTest("party-request"),
            "party requests use ReadyAlert's core alert audio path");
        Assert(!TrayApplicationContext.IsCoreSoundEventForSelfTest("chat"),
            "chat event kinds are not mixed into the core alert audio path");
    }

    private static void Assert(bool condition, string name)
    {
        if (!condition)
            throw new InvalidOperationException("v1.3.0 party alert self-test failed: " + name);
    }
}
