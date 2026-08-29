#nullable enable

#if BATTLEMENT_DITTO_DIAGNOSTICS
using System;
using System.Collections;
using UnityEngine;
using UnityEngine.Networking;

namespace Battlement
{
    internal sealed class DittoUnityWebRequestTransport : IDittoDeliveryTransport
    {
        private readonly MonoBehaviour owner;
        private readonly string sessionUrl;

        public DittoUnityWebRequestTransport(MonoBehaviour coroutineOwner, string baseUrl)
        {
            owner = coroutineOwner ?? throw new ArgumentNullException(nameof(coroutineOwner));
            sessionUrl = baseUrl?.TrimEnd('/') ?? throw new ArgumentNullException(nameof(baseUrl));
        }

        public void Send(DittoDeliveryRequest request, Action<DittoDeliveryResponse> completion) =>
            owner.StartCoroutine(Execute(request, completion));

        public void SendAfter(
            TimeSpan delay,
            DittoDeliveryRequest request,
            Action<DittoDeliveryResponse> completion
        ) => owner.StartCoroutine(ExecuteAfter(delay, request, completion));

        private IEnumerator ExecuteAfter(
            TimeSpan delay,
            DittoDeliveryRequest request,
            Action<DittoDeliveryResponse> completion
        )
        {
            yield return new WaitForSecondsRealtime((float)Math.Max(0, delay.TotalSeconds));
            yield return Execute(request, completion);
        }

        private IEnumerator Execute(
            DittoDeliveryRequest value,
            Action<DittoDeliveryResponse> completion
        )
        {
            using var request = new UnityWebRequest(
                sessionUrl + "/" + value.Path.TrimStart('/'),
                value.Method
            );
            request.uploadHandler = new UploadHandlerRaw(value.Body);
            request.downloadHandler = new DownloadHandlerBuffer();
            request.SetRequestHeader("Content-Type", value.ContentType);
            foreach ((string name, string headerValue) in value.Headers)
            {
                request.SetRequestHeader(name, headerValue);
            }
            yield return request.SendWebRequest();
            byte[] body = request.downloadHandler.data ?? Array.Empty<byte>();
            if (request.responseCode is >= 200 and < 300)
            {
                completion(new DittoDeliveryResponse.Accepted(body));
            }
            else if (request.responseCode > 0)
            {
                completion(new DittoDeliveryResponse.Rejected((int)request.responseCode, body));
            }
            else
            {
                completion(
                    new DittoDeliveryResponse.Uncertain(
                        request.error ?? "The Ditto request had no HTTP response."
                    )
                );
            }
        }
    }
}
#endif
