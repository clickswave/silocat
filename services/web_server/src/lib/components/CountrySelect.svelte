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
		<span class="arrow" class:open={isOpen}><Icon icon="ri:arrow-down-s-line" /></span>
	</button>

	{#if isOpen}
		<div class="dropdown-menu">
			<div class="search-box">
				<span class="search-icon"><Icon icon="ri:search-line" /></span>
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
								<span class="check-icon"><Icon icon="ri:check-line" /></span>
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
		padding: 0.75rem 0.95rem;
		background: var(--bg-input);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-sm);
		color: var(--text-primary);
		font-family: inherit;
		font-size: var(--fs-body);
		cursor: pointer;
		transition: background var(--dur) var(--ease), border-color var(--dur) var(--ease);
		text-align: left;

		&:hover {
			background: var(--bg-card-hover);
			border-color: var(--border-strong);
		}

		.text {
			flex: 1;
			white-space: nowrap;
			overflow: hidden;
			text-overflow: ellipsis;
		}

		.arrow {
			display: inline-flex;
			align-items: center;
			color: var(--text-muted);
			transition: transform 0.2s;
			&.open {
				transform: rotate(180deg);
			}
		}
	}

	.dropdown-menu {
		position: absolute;
		top: calc(100% + var(--space-1));
		left: 0;
		width: 100%;
		background: var(--bg-elevated);
		border: 1px solid var(--border-default);
		border-radius: var(--radius-md);
		box-shadow: var(--shadow-lg);
		z-index: 50;
		overflow: hidden;
		padding: var(--space-2);
		box-sizing: border-box;

		.search-box {
			position: relative;
			margin-bottom: var(--space-2);

			.search-icon {
				position: absolute;
				left: var(--space-2);
				top: 50%;
				transform: translateY(-50%);
				display: flex;
				align-items: center;
				font-size: 1rem;
				color: var(--text-muted);
				pointer-events: none;
			}

			input {
				width: 100%;
				padding: var(--space-2) var(--space-2) var(--space-2) var(--space-6);
				background: var(--bg-input);
				border: 1px solid var(--border-default);
				border-radius: var(--radius-sm);
				color: var(--text-primary);
				font-family: inherit;
				font-size: var(--fs-sm);
				box-sizing: border-box;
				transition: border-color var(--dur) var(--ease), box-shadow var(--dur) var(--ease);

				&::placeholder {
					color: var(--text-muted);
				}

				&:focus {
					outline: none;
					border-color: var(--primary);
					box-shadow: 0 0 0 3px var(--primary-glow);
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
				background: var(--border-strong);
				border-radius: var(--radius-pill);
			}

			li {
				margin-bottom: 2px;
			}

			.country-option {
				width: 100%;
				display: flex;
				justify-content: space-between;
				align-items: center;
				padding: var(--space-2) var(--space-3);
				background: transparent;
				border: none;
				color: var(--text-primary);
				font-family: inherit;
				border-radius: var(--radius-sm);
				cursor: pointer;
				text-align: left;
				font-size: var(--fs-sm);
				transition: background var(--dur) var(--ease);

				&:hover {
					background: var(--tint-soft);
				}

				&.selected {
					background: var(--accent-soft);
					color: var(--primary);
				}

				.check-icon {
					display: inline-flex;
					align-items: center;
					color: var(--primary);
				}
			}

			.no-results {
				padding: var(--space-3);
				text-align: center;
				color: var(--text-muted);
				font-size: var(--fs-sm);
			}
		}
	}
</style>
