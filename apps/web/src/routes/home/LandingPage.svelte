<script lang="ts">
	import Header from '$home/components/Header.svelte';
	import BlogHighlights from '$home/sections/BlogHighlights.svelte';
	import DevelopersReview from '$home/sections/DevelopersReview.svelte';
	import FAQ from '$home/sections/FAQ.svelte';
	import Features from '$home/sections/Features.svelte';
	import Footer from '$home/sections/Footer.svelte';
	import Hero from '$home/sections/Hero.svelte';
	import * as jsonLinks from '$lib/data/links.json';
	import { latestClientVersion } from '$lib/store';
	import { targetDownload } from '$lib/store';
	import { getOS } from '$lib/utils/getOS';
	import GhostContentAPI, { type PostsOrPages } from '@tryghost/content-api';
	import { onMount } from 'svelte';

	const GHOST_URL = 'https://gitbutler.ghost.io';
	const GHOST_KEY = '80bbdca8b933f3d98780c7cc1b';
	const GHOST_VERSION = 'v5.0';

	let posts = $state<PostsOrPages>();

	onMount(async () => {
		const api = GhostContentAPI({
			url: GHOST_URL,
			key: GHOST_KEY,
			version: GHOST_VERSION
		});
		posts = await api.posts.browse({ limit: 3, include: 'authors' });
	});

	onMount(() => {
		const os = getOS();

		if (os === 'macos') {
			targetDownload.set(jsonLinks.downloads.appleSilicon);
		} else if (os === 'linux') {
			targetDownload.set(jsonLinks.downloads.linuxDeb);
		} else if (os === 'windows') {
			targetDownload.set(jsonLinks.downloads.windowsMsi);
		} else {
			targetDownload.set(jsonLinks.downloads.appleSilicon);
		}

		// get actual latest version from https://app.gitbutler.com/latest_version
		fetch('https://app.gitbutler.com/latest_version')
			.then((res) => res.text())
			.then((data) => {
				latestClientVersion.set(data);
			});
	});
</script>

<Header />
<Hero />
<Features />
<DevelopersReview />
{#if posts}
	<BlogHighlights {posts} />
{/if}
<FAQ />
<Footer />
