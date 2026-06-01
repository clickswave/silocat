<script>
	import Icon from '@iconify/svelte';
	import { countries, getCountryName } from '$lib/countries';
	import { clickOutside } from '$lib/clickOutside';

	let { value = $bindable('') } = $props();

	let isOpen = $state(false);
	let searchTerm = $state('');
	let inputElement = $state(null);

	let filteredCountries = $derived(
		countries.filter((c) => c.name.toLowerCase().includes(searchTerm.toLowerCase()))
	);

	let selectedCountryName = $derived(getCountryName(value) || '');

	function toggleDropdown() {
		isOpen = !isOpen;
		if (isOpen) {
			searchTerm = '';
			setTimeout(() => inputElement?.focus(), 50);
		}
	}

	function selectCountry(code) {
		value = code;
		isOpen = false;
		searchTerm = '';
	}

	function closeDropdown() {
		isOpen = false;
	}
</script>

<div class="country-select" use:clickOutside={() => closeDropdown()}>
	<button type="button" class="select-trigger" onclick={toggleDropdown}>
		<span class="text">{selectedCountryName || 'Select a country'}</span>
		<Icon icon="ri:arrow-down-s-line" class="arrow {isOpen ? 'open' : ''}" />
	</button>

	{#if isOpen}
		<div class="dropdown-menu">
			<div class="search-box">
				<Icon icon="ri:search-line" class="search-icon" />
				<input
					bind:this={inputElement}
					type="text"
					placeholder="Search country..."
					bind:value={searchTerm}
					onclick={(e) => e.stopPropagation()}
				/>
			</div>
			<ul class="country-list">
				{#each filteredCountries as country}
					<li>
						<button
							type="button"
							class="country-option {value === country.code ? 'selected' : ''}"
							onclick={() => selectCountry(country.code)}
						>
							{country.name}
							{#if value === country.code}
								<Icon icon="ri:check-line" class="check-icon" />
							{/if}
						</button>
					</li>
				{:else}
					<li class="no-results">No countries found</li>
				{/each}
			</ul>
		</div>
	{/if}
</div>

<style lang="scss">
	.country-select {
		position: relative;
		width: 100%;
	}

	.select-trigger {
		width: 100%;
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 10px 14px;
		background: rgba(255, 255, 255, 0.03);
		border: 1px solid var(--border-default);
		border-radius: 8px;
		color: var(--text-primary);
		font-family: inherit;
		font-size: 0.95rem;
		cursor: pointer;
		transition: all 0.2s;
		text-align: left;

		&:hover {
			background: rgba(255, 255, 255, 0.05);
			border-color: var(--border-hover, #52525b);
		}

		.text {
			flex: 1;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}

		.arrow {
			color: var(--text-muted);
			transition: transform 0.2s;
			&.open {
				transform: rotate(180deg);
			}
		}
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + 4px);
		left: 0;
		width: 100%;
		background: var(--bg-card);
		border: 1px solid var(--border-default);
		border-radius: 8px;
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
		z-index: 50;
		overflow: hidden;
		padding: 8px;
		box-sizing: border-box;

		.search-box {
			position: relative;
			margin-bottom: 8px;

			.search-icon {
				position: absolute;
				left: 8px;
				top: 50%;
				transform: translateY(-50%);
				color: var(--text-muted);
				pointer-events: none;
			}

			input {
				width: 100%;
				padding: 8px 8px 8px 32px;
				background: rgba(255, 255, 255, 0.05);
				border: 1px solid transparent;
				border-radius: 6px;
				color: var(--text-primary);
				font-size: 0.9rem;
				box-sizing: border-box;

				&:focus {
					outline: none;
					background: rgba(255, 255, 255, 0.08);
					border-color: var(--primary);
				}
			}
		}

		.country-list {
			list-style: none;
			padding: 0;
			margin: 0;
			max-height: 200px;
			overflow-y: auto;

			/* Scrollbar styling */
			&::-webkit-scrollbar {
				width: 6px;
			}
			&::-webkit-scrollbar-track {
				background: transparent;
			}
			&::-webkit-scrollbar-thumb {
				background: var(--border-default);
				border-radius: 3px;
			}

			li {
				margin-bottom: 2px;
			}

			.country-option {
				width: 100%;
				display: flex;
				justify-content: space-between;
				align-items: center;
				padding: 8px 10px;
				background: transparent;
				border: none;
				color: var(--text-primary);
				border-radius: 4px;
				cursor: pointer;
				text-align: left;
				font-size: 0.9rem;
				transition: background 0.15s;

				&:hover {
					background: rgba(255, 255, 255, 0.05);
				}

				&.selected {
					background: rgba(255, 70, 85, 0.1);
					color: var(--primary);
				}

				.check-icon {
					color: var(--primary);
				}
			}

			.no-results {
				padding: 12px;
				text-align: center;
				color: var(--text-muted);
				font-size: 0.85rem;
			}
		}
	}
</style>
